use bitflags::bitflags;
use cached::proc_macro::cached;
use cached::UnboundCache;
use icu_properties::{maps, sets, GeneralCategory, GeneralCategoryGroup, Script};

use crate::consts::{COMMON_SAFE_ASCII_CHARACTERS, UTF8_MAXIMAL_ALLOCATION};
use crate::utils::{in_range, is_accentuated, unicode_range};

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

    pub fn is(&self, flag: MessDetectorCharFlags) -> bool {
        self.flags.contains(flag)
    }
}

#[cached(
    type = "UnboundCache<char, MessDetectorChar>",
    create = "{ UnboundCache::with_capacity(UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ character }"#
)]
fn new_mess_detector_character(character: char) -> MessDetectorChar {
    let mut flags = MessDetectorCharFlags::empty();
    // unicode information
    let gc = maps::general_category().get(character);

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
            && GeneralCategoryGroup::Control.contains(gc)
        {
            flags.insert(MessDetectorCharFlags::UNPRINTABLE);
        }

        // emoticon
        if sets::emoji_component().contains(character)
            || sets::emoji_modifier().contains(character)
            || sets::emoji_modifier_base().contains(character)
            || sets::emoji_presentation().contains(character)
        //    || sets::emoji().contains(character) //tests::md::test_mess_ratio fails
        {
            flags.insert(MessDetectorCharFlags::EMOTICON);
        }

        // separator
        if ['｜', '+', '<', '>'].contains(&character)
            || GeneralCategoryGroup::Separator.contains(gc)
            || matches!(
                gc,
                GeneralCategory::OtherPunctuation
                    | GeneralCategory::DashPunctuation
                    | GeneralCategory::ConnectorPunctuation
            )
        {
            flags.insert(MessDetectorCharFlags::SEPARATOR);
        }
    }

    // punctuation
    if GeneralCategoryGroup::Punctuation.contains(gc) {
        flags.insert(MessDetectorCharFlags::PUNCTUATION);
    }

    // symbol
    if GeneralCategoryGroup::Number.contains(gc)
        || GeneralCategoryGroup::Symbol.contains(gc)
        || in_range(range, &["Forms"])
    {
        flags.insert(MessDetectorCharFlags::SYMBOL);
    }

    match maps::script().get(character) {
        Script::Latin => flags.insert(MessDetectorCharFlags::LATIN), // latin
        Script::Han => flags.insert(MessDetectorCharFlags::CJK),     // han implies cjk
        Script::Hangul => flags.insert(MessDetectorCharFlags::HANGUL),
        Script::Katakana => flags.insert(MessDetectorCharFlags::KATAKANA),
        Script::Hiragana => flags.insert(MessDetectorCharFlags::HIRAGANA),
        Script::Thai => flags.insert(MessDetectorCharFlags::THAI),
        _ => {
            // ideographic() includes some characters such as vietnamese that might not be Han
            // but still be part of the expanded CJK(V) ideographs
            // if sets::ideographic().contains(character)
            if sets::unified_ideograph().contains(character) {
                flags.insert(MessDetectorCharFlags::CJK)
            }
        }
    }

    // accentuated
    if is_accentuated(character) {
        flags.insert(MessDetectorCharFlags::ACCENTUATED);
    }

    // create new object
    MessDetectorChar {
        character,
        flags,
        unicode_range: range,
    }
}
