use bitflags::bitflags;
use cached::proc_macro::cached;
use cached::UnboundCache;
use unic::char::property::EnumeratedCharProperty;
use unic::ucd::{GeneralCategory, Name};

use crate::consts::{COMMON_SAFE_ASCII_CHARACTERS, UTF8_MAXIMAL_ALLOCATION};
use crate::utils::unicode_range;

// Mess Plugin Char representation
// used to collect additional information about char
// and eliminate repeated calculations
#[derive(Copy, Clone, PartialEq)]
pub struct MessDetectorCharFlags(u32);

bitflags! {
    impl MessDetectorCharFlags: u32 {
        const WHITESPACE        = 0b0000_0000_0000_0000_0000_0000_0000_0001;
        const UNPRINTABLE       = 0b0000_0000_0000_0000_0000_0000_0000_0010;
        const SYMBOL            = 0b0000_0000_0000_0000_0000_0000_0000_0100;
        const EMOTICON          = 0b0000_0000_0000_0000_0000_0000_0000_1000;
        const COMMON_SAFE       = 0b0000_0000_0000_0000_0000_0000_0001_0000;
        const WEIRD_SAFE        = 0b0000_0000_0000_0000_0000_0000_0010_0000;
        const PUNCTUATION       = 0b0000_0000_0000_0000_0000_0000_0100_0000;
        const SEPARATOR         = 0b0000_0000_0000_0000_0000_0000_1000_0000;
        const ASCII             = 0b0000_0000_0000_0000_0000_0001_0000_0000;
        const ASCII_ALPHABETIC  = 0b0000_0000_0000_0000_0000_0010_0000_0000;
        const ASCII_GRAPHIC     = 0b0000_0000_0000_0000_0000_0100_0000_0000;
        const ASCII_DIGIT       = 0b0000_0000_0000_0000_0000_1000_0000_0000;
        const LATIN             = 0b0000_0000_0000_0000_0001_0000_0000_0000;
        const ALPHABETIC        = 0b0000_0000_0000_0000_0010_0000_0000_0000;
        const ACCENTUATED       = 0b0000_0000_0000_0000_0100_0000_0000_0000;
        const CJK               = 0b0000_0000_0000_0000_1000_0000_0000_0000;
        const HANGUL            = 0b0000_0000_0000_0001_0000_0000_0000_0000;
        const KATAKANA          = 0b0000_0000_0000_0010_0000_0000_0000_0000;
        const HIRAGANA          = 0b0000_0000_0000_0100_0000_0000_0000_0000;
        const THAI              = 0b0000_0000_0000_1000_0000_0000_0000_0000;
        const CASE_VARIABLE     = 0b0000_0000_0001_0000_0000_0000_0000_0000;
        const LOWERCASE         = 0b0000_0000_0010_0000_0000_0000_0000_0000;
        const UPPERCASE         = 0b0000_0000_0100_0000_0000_0000_0000_0000;
        const NUMERIC           = 0b0000_0000_1000_0000_0000_0000_0000_0000;
    }
}

#[derive(Copy, Clone)]
pub(crate) struct MessDetectorChar {
    pub character: char,
    pub flags: MessDetectorCharFlags,
    pub unicode_range: Option<&'static str>,
}

impl PartialEq for MessDetectorChar {
    fn eq(&self, other: &Self) -> bool {
        self.character == other.character
    }
}

impl MessDetectorChar {
    pub fn new(character: char) -> Self {
        new_mess_detector_character(character)
    }
    pub fn in_category(
        category: &str,
        range: Option<&str>,
        categories_exact: &[&str],
        categories_partial: &[&str],
        ranges_partial: &[&str],
    ) -> bool {
        // unicode category part
        if categories_exact.contains(&category)
            || categories_partial.iter().any(|&cp| category.contains(cp))
        {
            return true;
        }
        // unicode range part
        if !ranges_partial.is_empty() {
            if let Some(range) = range {
                return ranges_partial.iter().any(|&r| range.contains(r));
            }
        }
        false
    }

    pub fn in_description(name: Option<Name>, patterns: &[&str]) -> bool {
        name.is_some_and(|description| {
            patterns
                .iter()
                .any(|&s| description.to_string().contains(s))
        })
    }

    pub fn is(&self, flag: MessDetectorCharFlags) -> bool {
        self.flags.contains(flag)
    }
}

#[cached(
    type = "UnboundCache<char, MessDetectorChar>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ character }"#
)]
fn new_mess_detector_character(character: char) -> MessDetectorChar {
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
