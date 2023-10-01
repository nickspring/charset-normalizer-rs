#![allow(unused_variables)]

use crate::consts::COMMON_SAFE_ASCII_CHARACTERS;
use crate::utils::{
    is_accentuated, is_case_variable, is_cjk, is_emoticon, is_hangul, is_hiragana,
    is_katakana, is_latin, is_punctuation, is_separator, is_suspiciously_successive_range,
    is_symbol, is_thai, is_unprintable, remove_accent, unicode_range,
};
use cached::proc_macro::cached;
use log::trace;
use ordered_float::OrderedFloat;

//
// Mess detection module
//

// Base abstract trait used for mess detection plugins.
// All detectors MUST extend and implement given methods.
trait MessDetectorPlugin {
    // Name of plugin
    fn name(&self) -> &str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    // Determine if given character should be fed in
    fn eligible(&self, character: &char) -> bool;

    // The main routine to be executed upon character.
    // Insert the logic in witch the text would be considered chaotic.
    fn feed(&mut self, character: &char);

    // Compute the chaos ratio based on what your feed() has seen.
    // Must NOT be lower than 0.; No restriction gt 0.
    fn ratio(&self) -> f32;
}

//
// TooManySymbolOrPunctuationPlugin implementation
//
#[derive(Default)]
struct TooManySymbolOrPunctuationPlugin {
    punctuation_count: u64,
    symbol_count: u64,
    character_count: u64,
    last_printable_char: Option<char>,
}

impl MessDetectorPlugin for TooManySymbolOrPunctuationPlugin {
    fn eligible(&self, character: &char) -> bool {
        !is_unprintable(character)
    }
    fn feed(&mut self, character: &char) {
        self.character_count += 1;
        if (self.last_printable_char.is_none() || *character != self.last_printable_char.unwrap())
            && !COMMON_SAFE_ASCII_CHARACTERS.contains(*character)
        {
            if is_punctuation(character) {
                self.punctuation_count += 1;
            } else if !character.is_numeric() && is_symbol(character) && !is_emoticon(character) {
                self.symbol_count += 2;
            }
        }
        self.last_printable_char = Some(*character);
    }
    fn ratio(&self) -> f32 {
        if self.character_count == 0 {
            return 0.0;
        }
        let ratio_of_punctuation: f32 = (self.punctuation_count as f32 + self.symbol_count as f32)
            / (self.character_count as f32);

        if ratio_of_punctuation >= 0.3 {
            ratio_of_punctuation
        } else {
            0.0
        }
    }
}

//
// TooManyAccentuatedPlugin implementation
//

#[derive(Default)]
struct TooManyAccentuatedPlugin {
    character_count: u64,
    accentuated_count: u64,
}

impl MessDetectorPlugin for TooManyAccentuatedPlugin {
    fn eligible(&self, character: &char) -> bool {
        character.is_alphabetic()
    }
    fn feed(&mut self, character: &char) {
        self.character_count += 1;
        if is_accentuated(character) {
            self.accentuated_count += 1
        }
    }
    fn ratio(&self) -> f32 {
        if self.character_count < 8 {
            return 0.0;
        }
        let ratio_of_accentuation: f32 =
            self.accentuated_count as f32 / self.character_count as f32;
        if ratio_of_accentuation >= 0.35 {
            ratio_of_accentuation
        } else {
            0.0
        }
    }
}

//
// UnprintablePlugin implementation
//

#[derive(Default)]
struct UnprintablePlugin {
    character_count: u64,
    unprintable_count: u64,
}

impl MessDetectorPlugin for UnprintablePlugin {
    fn eligible(&self, character: &char) -> bool {
        true
    }
    fn feed(&mut self, character: &char) {
        if is_unprintable(character) {
            self.unprintable_count += 1;
        }
        self.character_count += 1
    }
    fn ratio(&self) -> f32 {
        if self.character_count == 0 {
            return 0.0;
        }
        (self.unprintable_count as f32 * 8.0) / self.character_count as f32
    }
}

//
// SuspiciousDuplicateAccentPlugin implementation
//
#[derive(Default)]
struct SuspiciousDuplicateAccentPlugin {
    character_count: u64,
    successive_count: u64,
    last_latin_character: Option<char>,
}

impl MessDetectorPlugin for SuspiciousDuplicateAccentPlugin {
    fn eligible(&self, character: &char) -> bool {
        character.is_alphabetic() && is_latin(character)
    }
    fn feed(&mut self, character: &char) {
        self.character_count += 1;
        if self.last_latin_character.is_some()
            && is_accentuated(character)
            && is_accentuated(&self.last_latin_character.unwrap())
        {
            if character.is_uppercase() && self.last_latin_character.unwrap().is_uppercase() {
                self.successive_count += 1;
            }

            // Worse if its the same char duplicated with different accent.
            if remove_accent(character) == remove_accent(&self.last_latin_character.unwrap()) {
                self.successive_count += 1;
            }
        }
        self.last_latin_character = Some(*character);
    }
    fn ratio(&self) -> f32 {
        if self.character_count == 0 {
            return 0.0;
        }
        (self.successive_count as f32 * 2.0) / self.character_count as f32
    }
}

//
// SuspiciousRangePlugin implementation
//
#[derive(Default)]
struct SuspiciousRangePlugin {
    character_count: u64,
    suspicious_successive_range_count: u64,
    last_printable_char: Option<char>,
}

impl MessDetectorPlugin for SuspiciousRangePlugin {
    fn eligible(&self, character: &char) -> bool {
        !is_unprintable(character)
    }
    fn feed(&mut self, character: &char) {
        self.character_count += 1;

        if character.is_whitespace()
            || is_punctuation(character)
            || COMMON_SAFE_ASCII_CHARACTERS.contains(*character)
        {
            self.last_printable_char = None;
            return;
        }

        if self.last_printable_char.is_none() {
            self.last_printable_char = Some(*character);
            return;
        }

        let tmp_a = &self.last_printable_char.unwrap();
        let unicode_range_a = unicode_range(tmp_a);
        let unicode_range_b = unicode_range(character);

        if is_suspiciously_successive_range(unicode_range_a, unicode_range_b) {
            self.suspicious_successive_range_count += 1;
        }

        self.last_printable_char = Some(*character);
    }
    fn ratio(&self) -> f32 {
        if self.character_count == 0 {
            return 0.0;
        }

        let ratio_of_suspicious_range_usage: f32 =
            ((self.suspicious_successive_range_count as f32) * 2.0) / self.character_count as f32;

        if ratio_of_suspicious_range_usage < 0.1 {
            return 0.0;
        }
        ratio_of_suspicious_range_usage
    }
}

//
// SuperWeirdWordPlugin implementation
//

struct SuperWeirdWordPlugin {
    character_count: u64,
    word_count: u64,
    bad_word_count: u64,
    foreign_long_count: u64,
    is_current_word_bad: bool,
    foreign_long_watch: bool,
    bad_character_count: u64,
    buffer_accent_count: u64,
    buffer: String,
}

impl Default for SuperWeirdWordPlugin {
    fn default() -> Self {
        SuperWeirdWordPlugin {
            word_count: 0,
            bad_word_count: 0,
            foreign_long_count: 0,
            is_current_word_bad: false,
            foreign_long_watch: false,
            character_count: 0,
            bad_character_count: 0,
            buffer: "".to_string(),
            buffer_accent_count: 0,
        }
    }
}

impl MessDetectorPlugin for SuperWeirdWordPlugin {
    fn eligible(&self, character: &char) -> bool {
        true
    }
    fn feed(&mut self, character: &char) {
        if character.is_ascii_alphabetic() {
            self.buffer.push(*character);
            if is_accentuated(character) {
                self.buffer_accent_count += 1;
            }
            if !self.foreign_long_watch
                && (!is_latin(character) || is_accentuated(character))
                && !is_cjk(character)
                && !is_hangul(character)
                && !is_katakana(character)
                && !is_hiragana(character)
                && !is_thai(character)
            {
                self.foreign_long_watch = true;
            }
            return;
        }
        if self.buffer.is_empty() {
            return;
        }

        if character.is_whitespace() || is_punctuation(character) || is_separator(character) {
            self.word_count += 1;
            let buffer_length = self.buffer.len();
            self.character_count += buffer_length as u64;

            if buffer_length >= 4 {
                if (self.buffer_accent_count as f32 / buffer_length as f32) > 0.34 {
                    self.is_current_word_bad = true;
                }

                // Word/Buffer ending with an upper case accentuated letter are so rare,
                // that we will consider them all as suspicious. Same weight as foreign_long suspicious.
                let last_char = self.buffer.chars().last().unwrap();
                if is_accentuated(&last_char) && last_char.is_uppercase() {
                    self.foreign_long_count += 1;
                    self.is_current_word_bad = true;
                }
            }
            if buffer_length >= 24 && self.foreign_long_watch {
                let uppercase_count = self.buffer.chars().filter(|c| c.is_uppercase()).count();
                let mut probable_camel_cased: bool = false;

                if uppercase_count > 0 && (uppercase_count as f32 / buffer_length as f32) <= 0.3 {
                    probable_camel_cased = true;
                }

                if !probable_camel_cased {
                    self.foreign_long_count += 1;
                    self.is_current_word_bad = true;
                }
            }

            if self.is_current_word_bad {
                self.bad_word_count += 1;
                self.bad_character_count += self.buffer.len() as u64;
                self.is_current_word_bad = false;
            }

            self.foreign_long_watch = false;
            self.buffer.clear();
            self.buffer_accent_count = 0;
        } else if !"<>-=~|_".contains(*character)
            && !character.is_ascii_digit()
            && is_symbol(character)
        {
            self.is_current_word_bad = true;
            self.buffer.push(*character);
        }
    }
    fn ratio(&self) -> f32 {
        if self.word_count <= 10 && self.foreign_long_count == 0 {
            return 0.0;
        }
        self.bad_character_count as f32 / self.character_count as f32
    }
}

//
// CjkInvalidStopPlugin implementation
//
// GB(Chinese) based encoding often render the stop incorrectly when the content does not fit and
// can be easily detected. Searching for the overuse of '丅' and '丄'.
#[derive(Default)]
struct CjkInvalidStopPlugin {
    wrong_stop_count: u64,
    cjk_character_count: u64,
}

impl MessDetectorPlugin for CjkInvalidStopPlugin {
    fn eligible(&self, character: &char) -> bool {
        true
    }
    fn feed(&mut self, character: &char) {
        if "丅丄".contains(*character) {
            self.wrong_stop_count += 1;
            return;
        }
        if is_cjk(character) {
            self.cjk_character_count += 1;
        }
    }
    fn ratio(&self) -> f32 {
        if self.cjk_character_count < 16 {
            return 0.0;
        }
        self.wrong_stop_count as f32 / self.cjk_character_count as f32
    }
}

//
// ArchaicUpperLowerPlugin implementation
//

struct ArchaicUpperLowerPlugin {
    buf: bool,
    current_ascii_only: bool,
    character_count_since_last_sep: u64,
    successive_upper_lower_count: u64,
    successive_upper_lower_count_final: u64,
    character_count: u64,
    last_alpha_seen: Option<char>,
}

impl Default for ArchaicUpperLowerPlugin {
    fn default() -> Self {
        ArchaicUpperLowerPlugin {
            buf: false,
            current_ascii_only: true,
            character_count_since_last_sep: 0,
            successive_upper_lower_count: 0,
            successive_upper_lower_count_final: 0,
            character_count: 0,
            last_alpha_seen: None,
        }
    }
}

impl MessDetectorPlugin for ArchaicUpperLowerPlugin {
    fn eligible(&self, character: &char) -> bool {
        true
    }
    fn feed(&mut self, character: &char) {
        if !(character.is_alphabetic() && is_case_variable(character))
            && self.character_count_since_last_sep > 0
        {
            if self.character_count_since_last_sep <= 64
                && !character.is_ascii_digit()
                && !self.current_ascii_only
            {
                self.successive_upper_lower_count_final += self.successive_upper_lower_count;
            }

            self.successive_upper_lower_count = 0;
            self.character_count_since_last_sep = 0;
            self.last_alpha_seen = None;
            self.buf = false;
            self.character_count += 1;
            self.current_ascii_only = true;

            return;
        }

        if self.current_ascii_only && !character.is_ascii() {
            self.current_ascii_only = false;
        }

        if let Some(tmp_last_alpha) = self.last_alpha_seen {
            if (character.is_uppercase() && tmp_last_alpha.is_lowercase())
                || (character.is_lowercase() && tmp_last_alpha.is_uppercase())
            {
                if self.buf {
                    self.successive_upper_lower_count += 2;
                    self.buf = false;
                } else {
                    self.buf = true;
                }
            } else {
                self.buf = false;
            }
        }

        self.character_count += 1;
        self.character_count_since_last_sep += 1;
        self.last_alpha_seen = Some(*character);
    }
    fn ratio(&self) -> f32 {
        if self.character_count == 0 {
            return 0.0;
        }
        self.successive_upper_lower_count_final as f32 / self.character_count as f32
    }
}

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
        for detector in &mut *detectors {
            if detector.eligible(&ch) {
                detector.feed(&ch);
            }
        }

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
