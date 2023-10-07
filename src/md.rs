use crate::consts::{COMMON_SAFE_ASCII_CHARACTERS, UTF8_MAXIMAL_ALLOCATION};
use crate::utils::unicode_range;
use cached::proc_macro::cached;
use cached::UnboundCache;
use log::trace;
use ordered_float::OrderedFloat;
use unic::char::property::EnumeratedCharProperty;
use unic::ucd::{GeneralCategory, Name};

pub(crate) mod plugins;
pub(crate) mod structs;
use plugins::{
    ArchaicUpperLowerPlugin, CjkInvalidStopPlugin, MessDetectorPlugin, SuperWeirdWordPlugin,
    SuspiciousDuplicateAccentPlugin, SuspiciousRangePlugin, TooManyAccentuatedPlugin,
    TooManySymbolOrPunctuationPlugin, UnprintablePlugin,
};
use structs::MessDetectorChar;

use self::structs::MessDetectorCharFlags;

//
// Mess detection module
//

// Compute a mess ratio given a decoded bytes sequence. The maximum threshold does stop the computation earlier.
#[cached(size = 2048)]
pub(crate) fn mess_ratio(
    decoded_sequence: String,
    maximum_threshold: Option<OrderedFloat<f32>>,
) -> f32 {
    let maximum_threshold = f32::from(maximum_threshold.unwrap_or(OrderedFloat(0.2)));
    let mut detectors: Vec<Box<dyn MessDetectorPlugin>> = vec![
        Box::<TooManySymbolOrPunctuationPlugin>::default(),
        Box::<TooManyAccentuatedPlugin>::default(),
        Box::<UnprintablePlugin>::default(),
        Box::<SuspiciousRangePlugin>::default(),
        Box::<SuspiciousDuplicateAccentPlugin>::default(),
        Box::<SuperWeirdWordPlugin>::default(),
        Box::<CjkInvalidStopPlugin>::default(),
        Box::<ArchaicUpperLowerPlugin>::default(),
    ];

    let length = decoded_sequence.chars().count();
    let mut mean_mess_ratio: f32 = 0.0;
    let intermediary_mean_mess_ratio_calc: usize = match length {
        0..=510 => 32,
        511..=1023 => 64,
        _ => 128,
    };
    // Traverse through chars and detectors
    for (index, ch) in decoded_sequence
        .chars()
        .chain(std::iter::once('\n'))
        .enumerate()
    {
        let mess_char = MessDetectorChar::new(ch);
        detectors
            .iter_mut()
            .filter(|detector| detector.eligible(&mess_char))
            .for_each(|detector| detector.feed(&mess_char));

        if (index > 0 && index.rem_euclid(intermediary_mean_mess_ratio_calc) == 0)
            || index == length
        {
            mean_mess_ratio = detectors.iter().map(|x| x.ratio()).sum();
            if mean_mess_ratio >= maximum_threshold {
                break;
            }
        }
    }

    trace!(
        "Mess-detector extended-analysis start: \
        intermediary_mean_mess_ratio_calc={}, \
        mean_mess_ratio={}, \
        maximum_threshold={}",
        intermediary_mean_mess_ratio_calc,
        mean_mess_ratio,
        maximum_threshold,
    );

    /*if decoded_sequence.len() > 16 {
        trace!(
            "Chunk: {} ..... {}",
            &decoded_sequence[..decoded_sequence
                .char_indices()
                .nth(16)
                .map(|(i, _)| i)
                .unwrap_or(decoded_sequence.chars().count())],
            &decoded_sequence[decoded_sequence
                .char_indices()
                .nth(decoded_sequence.chars().count() - 16)
                .map(|(i, _)| i)
                .unwrap_or(decoded_sequence.chars().count())..],
        );
    }
     */

    for detector in detectors {
        if detector.ratio() > 0.0 {
            trace!("{} produces ratio: {}", detector.name(), detector.ratio());
        }
    }
    trace!("===");

    mean_mess_ratio
}

#[cached(
    type = "UnboundCache<char, MessDetectorChar>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ character }"#
)]
pub fn new_mess_detector_character(character: char) -> MessDetectorChar {
    let mut flags = MessDetectorCharFlags::empty();

    // PLEASE NOTE! In case of idiomatic refactoring
    // take in account performance. Sometimes match could be used but it
    // will require calculate all conditions and can decrease performance
    // in comparison to usual if then else

    // ascii probing
    if character.is_ascii() {
        flags.insert(MessDetectorCharFlags::ASCII);
        if character.is_ascii_graphic() {
            flags.insert(MessDetectorCharFlags::ASCII_GRAPHIC);
            if character.is_ascii_alphabetic() {
                flags.insert(MessDetectorCharFlags::ASCII_ALPHABETIC);
            } else if character.is_ascii_digit() {
                flags.insert(MessDetectorCharFlags::ASCII_DIGIT);
            }
        }
    }

    // unicode information
    let name = Name::of(character);
    let category = GeneralCategory::of(character).abbr_name();
    let range = unicode_range(character);

    // whitespace
    if character.is_whitespace() {
        flags.insert(MessDetectorCharFlags::WHITESPACE);
        flags.insert(MessDetectorCharFlags::SEPARATOR);
    } else {
        // safe symbols (non-whitespace)
        if COMMON_SAFE_ASCII_CHARACTERS.contains(character) {
            flags.insert(MessDetectorCharFlags::COMMON_SAFE);
        }
        if "<>-=~|_".contains(character) {
            flags.insert(MessDetectorCharFlags::WEIRD_SAFE);
        }

        // numeric
        if flags.contains(MessDetectorCharFlags::ASCII_DIGIT) || character.is_numeric() {
            flags.insert(MessDetectorCharFlags::NUMERIC);
        } else if flags.contains(MessDetectorCharFlags::ASCII_ALPHABETIC)
            || character.is_alphabetic()
        {
            // alphabetic
            flags.insert(MessDetectorCharFlags::ALPHABETIC);
            if character.is_lowercase() {
                flags.insert(MessDetectorCharFlags::LOWERCASE);
                flags.insert(MessDetectorCharFlags::CASE_VARIABLE);
            } else if character.is_uppercase() {
                flags.insert(MessDetectorCharFlags::UPPERCASE);
                flags.insert(MessDetectorCharFlags::CASE_VARIABLE);
            }
        } else if !flags.contains(MessDetectorCharFlags::ASCII_GRAPHIC)
            && !['\x1A', '\u{FEFF}'].contains(&character)
            && MessDetectorChar::in_category(category, range, &["Cc"], &[], &["Control character"])
        {
            flags.insert(MessDetectorCharFlags::UNPRINTABLE);
        }

        // emoticon
        if MessDetectorChar::in_category(category, range, &[], &[], &["Emoticons"]) {
            flags.insert(MessDetectorCharFlags::EMOTICON);
        }

        // separator
        if ['｜', '+', '<', '>'].contains(&character)
            || MessDetectorChar::in_category(category, range, &["Po", "Pd", "Pc"], &["Z"], &[])
        {
            flags.insert(MessDetectorCharFlags::SEPARATOR);
        }
    }

    // punctuation
    if MessDetectorChar::in_category(category, range, &[], &["P"], &["Punctuation"]) {
        flags.insert(MessDetectorCharFlags::PUNCTUATION);
    }

    // symbol
    if MessDetectorChar::in_category(category, range, &[], &["N", "S"], &["Forms"]) {
        flags.insert(MessDetectorCharFlags::SYMBOL);
    }

    // latin
    if MessDetectorChar::in_description(name, &["LATIN"]) {
        flags.insert(MessDetectorCharFlags::LATIN);
    } else {
        // cjk
        if MessDetectorChar::in_description(name, &["CJK"]) {
            flags.insert(MessDetectorCharFlags::CJK);
        }
        // hangul
        if MessDetectorChar::in_description(name, &["HANGUL"]) {
            flags.insert(MessDetectorCharFlags::HANGUL);
        }
        // katakana
        if MessDetectorChar::in_description(name, &["KATAKANA"]) {
            flags.insert(MessDetectorCharFlags::KATAKANA);
        }
        // hiragana
        if MessDetectorChar::in_description(name, &["HIRAGANA"]) {
            flags.insert(MessDetectorCharFlags::HIRAGANA);
        }
        // thai
        if MessDetectorChar::in_description(name, &["THAI"]) {
            flags.insert(MessDetectorCharFlags::THAI);
        }
    }

    // accentuated
    if MessDetectorChar::in_description(
        name,
        &[
            "WITH GRAVE",
            "WITH ACUTE",
            "WITH CEDILLA",
            "WITH DIAERESIS",
            "WITH CIRCUMFLEX",
            "WITH TILDE",
        ],
    ) {
        flags.insert(MessDetectorCharFlags::ACCENTUATED);
    }

    // create new object
    MessDetectorChar {
        character,
        flags,
        unicode_range: range,
    }
}
