use bitflags::bitflags;
use unic::ucd::Name;

use super::new_mess_detector_character;

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
pub struct MessDetectorChar {
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
