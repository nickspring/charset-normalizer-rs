use crate::assets::LANGUAGES;
use crate::consts::{
    ENCODING_MARKS, IANA_SUPPORTED_SIMILAR, RE_POSSIBLE_ENCODING_INDICATION,
    UNICODE_RANGES_COMBINED, UNICODE_SECONDARY_RANGE_KEYWORD,
};
use crate::enc::{Encoding, IsChunk, WantDecode};
use crate::entity::Language;
use std::cmp::Ordering;

use ahash::{HashSet, HashSetExt};
use icu_normalizer::DecomposingNormalizer;
use unicode_names2::name;

#[cfg(any(test, feature = "performance"))]
use std::path::{Path, PathBuf};

// Utils module

#[inline]
pub(crate) fn in_range(range: Option<&str>, ranges_partial: &[&str]) -> bool {
    // unicode range part
    if !ranges_partial.is_empty() {
        if let Some(range) = range {
            return ranges_partial.iter().any(|&r| range.contains(r));
        }
    }
    false
}

#[inline]
pub(crate) fn in_description(character: char, patterns: &[&str]) -> bool {
    name(character).is_some_and(|ucd_name| {
        let ucd_name = ucd_name.to_string();
        patterns.iter().any(|&s| ucd_name.contains(s))
    })
}

pub(crate) fn is_accentuated(character: char) -> bool {
    let patterns = [
        "WITH GRAVE",
        "WITH ACUTE",
        "WITH CEDILLA",
        "WITH DIAERESIS",
        "WITH CIRCUMFLEX",
        "WITH TILDE",
    ];
    in_description(character, &patterns)
}

pub(crate) fn is_unicode_range_secondary(range_name: &str) -> bool {
    UNICODE_SECONDARY_RANGE_KEYWORD
        .iter()
        .any(|&s| range_name.contains(s))
}

// Retrieve the Unicode range official name from a single character
pub(crate) fn unicode_range(character: char) -> Option<&'static str> {
    let char_code = character as u32;

    let index = UNICODE_RANGES_COMBINED
        .binary_search_by(|(_, range)| {
            if char_code < *range.start() {
                Ordering::Greater
            } else if char_code > *range.end() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .ok()?;
    UNICODE_RANGES_COMBINED
        .get(index)
        .map(|(name, _range)| *name)
}

pub(crate) fn range_scan(decoded_sequence: &str) -> HashSet<String> {
    let (lower, upper) = decoded_sequence.chars().size_hint();
    let mut result: HashSet<String> = HashSet::with_capacity(upper.unwrap_or(lower));
    result.extend(
        decoded_sequence
            .chars()
            .filter_map(|ch| unicode_range(ch).map(|r| r.to_string())),
    );
    result // decoded_sequence.chars().filter_map(|ch| unicode_range(&ch).map(|r| r.to_string())).collect()
}

pub(crate) fn remove_accent(ch: char) -> char {
    DecomposingNormalizer::new_nfd() //initialize decomposer
        .normalize(ch.to_string().as_str()) //normalize into String
        .chars()
        .next() // retrieve first component(unaccented char)
        .unwrap_or(ch) //if fail, return the original char
}

// Verify is a specific encoding is a multi byte one based on it IANA name
pub fn is_multi_byte_encoding(name: &str) -> bool {
    Encoding::by_name(name)
        .map(|enc| enc.is_multi_byte_encoding())
        .unwrap_or(false)
}

// Try to detect multibyte encoding by signature
pub(crate) fn identify_sig_or_bom(sequence: &[u8]) -> (Option<&Encoding>, Option<&[u8]>) {
    ENCODING_MARKS
        .iter()
        .find(|&(_, enc_sig)| sequence.starts_with(enc_sig))
        .map_or(
            (None, None),
            |(enc_name, enc_sig)| match Encoding::by_name(enc_name) {
                Some(enc) => (Some(enc), Some(*enc_sig)),
                None => (None, Some(*enc_sig)),
            },
        )
}

pub(crate) fn is_cp_similar(iana_name_a: &str, iana_name_b: &str) -> bool {
    IANA_SUPPORTED_SIMILAR
        .get(iana_name_a)
        .map(|candidates| candidates.contains(&iana_name_b))
        .unwrap_or(false)
}

// Extract using ASCII-only decoder any specified encoding in the first n-bytes.
pub(crate) fn any_specified_encoding(sequence: &[u8], search_zone: usize) -> Option<String> {
    let test_string = &sequence[0..search_zone.min(sequence.len())];

    RE_POSSIBLE_ENCODING_INDICATION
        .captures_iter(test_string)
        .map(|c| c.extract())
        .find_map(|(_, [specified_encoding])| {
            std::str::from_utf8(specified_encoding)
                .ok()
                .and_then(Encoding::by_name)
        })
        .map(|found_iana| found_iana.to_string())
}

// Calculate similarity of two single byte encodings
#[allow(dead_code)]
pub(crate) fn cp_similarity(iana_name_a: &str, iana_name_b: &str) -> f32 {
    // we don't want to compare multi-byte encodings
    if is_multi_byte_encoding(iana_name_a) || is_multi_byte_encoding(iana_name_b) {
        return 0.0;
    }

    if let (Some(encoder_a), Some(encoder_b)) = (
        Encoding::by_name(iana_name_a),
        Encoding::by_name(iana_name_b),
    ) {
        let character_match_count = (1..255u8)
            .filter(|&ch| {
                let res_a = encoder_a.decode(&[ch], WantDecode::Yes, IsChunk::No).ok();
                let res_b = encoder_b.decode(&[ch], WantDecode::Yes, IsChunk::No).ok();
                res_a.is_some() && res_a == res_b //check that they aren't none and equal
            })
            .count();
        return character_match_count as f32 / 254.0;
    }
    0.0 // Return 0.0 if encoders could not be retrieved.
}

/// Encode string to vec of bytes with specified encoding
pub fn encode(input: &str, to_encoding: &str, ignore_errors: bool) -> Result<Vec<u8>, String> {
    match Encoding::by_name(to_encoding) {
        Some(enc) => enc.encode(input, ignore_errors),
        None => Err(format!("Encoding '{}' not found", to_encoding)),
    }
}

// Determine if two Unicode range seen next to each other can be considered as suspicious.
pub(crate) fn is_suspiciously_successive_range(
    range_a: Option<&'static str>,
    range_b: Option<&'static str>,
) -> bool {
    if let (Some(range_a), Some(range_b)) = (range_a, range_b) {
        if range_a == range_b
            || [range_a, range_b].iter().all(|x| x.contains("Latin"))
            || [range_a, range_b].iter().any(|x| x.contains("Emoticons"))
        {
            return false;
        }

        // Latin characters can be accompanied with a combining diacritical mark
        // eg. Vietnamese.
        if [range_a, range_b].iter().any(|x| x.contains("Latin"))
            && [range_a, range_b].iter().any(|x| x.contains("Combining"))
        {
            return false;
        }

        // keywords intersection
        let set_a: HashSet<_> = range_a.split_whitespace().collect();
        let set_b: HashSet<_> = range_b.split_whitespace().collect();

        if set_a
            .intersection(&set_b)
            .any(|elem| !UNICODE_SECONDARY_RANGE_KEYWORD.contains(elem))
        {
            return false;
        }

        // Japanese exception
        let jp_ranges = ["Hiragana", "Katakana"];
        match (
            jp_ranges.contains(&range_a),                            // has_jp_a
            jp_ranges.contains(&range_b),                            // has_jp_b
            [range_a, range_b].iter().any(|x| x.contains("CJK")),    // has_cjk
            [range_a, range_b].iter().any(|x| x.contains("Hangul")), // has_hangul
            [range_a, range_b]
                .iter()
                .any(|x| x.contains("Punctuation") || x.contains("Forms")), // has_punct_or_forms
            [range_a, range_b].contains(&"Basic Latin"),  // is_any_basic_latin
        ) {
            (true, true, _, _, _, _) // both are japanese
            | (true, _, true, _, _, _) | (_, true, true, _, _, _) //either is japanese and either contains CJK
            | (_, _, true, true, _, _) // either has both CJK and Hanguls
            | (_, _, true, _, true, _) // either has chinese and dedicated punctuation and separators
            | (_, _, _, true, _, true) // either has hangul and basic latin
            => return false,
            _ => {} // All other combinations
        }
    }
    true // if either range is none or edge cases never triggers, return true
}

// Get data for specified language
pub(crate) fn get_language_data(language: &Language) -> Result<(&'static str, bool, bool), String> {
    for (iterated_language, characters, has_accents, pure_latin) in LANGUAGES.iter() {
        if iterated_language == language {
            return Ok((characters, *has_accents, *pure_latin));
        }
    }
    Err(String::from("Language wasn't found"))
}

// ascii in encodings means windows-1252 codepage with supports diacritis
// because of this we will check additionally it with is_ascii method
pub(super) fn is_invalid_chunk(
    decoded_chunk_result: &Result<String, String>,
    encoding_iana: &Encoding,
) -> bool {
    decoded_chunk_result.is_err()
        || (encoding_iana.name() == "ascii"
            && !decoded_chunk_result.as_ref().is_ok_and(|s| s.is_ascii()))
}

// Get large datasets
#[cfg(any(test, feature = "performance"))]
fn collect_large_sets(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();

            if path.is_dir() {
                // Recursively collect files in subdirectories
                let subdirectory_files = collect_large_sets(&path);
                files.extend(subdirectory_files);
            } else {
                // Add the file to the list if it's a regular file
                files.push(path);
            }
        }
    }
    files
}

// Get large datasets
#[cfg(any(test, feature = "performance"))]
pub fn get_large_test_datasets() -> Result<Vec<(String, Vec<String>)>, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/data/largesets/");

    match std::fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => Ok(collect_large_sets(&path)
            .iter()
            .filter_map(|set| {
                let path = set.to_str()?;
                let encoding: Vec<&str> = path.split('/').collect();
                let encoding: Vec<String> = encoding
                    .get(encoding.len().checked_sub(2)?)?
                    .split(',')
                    .map(|s| s.to_string())
                    .collect();
                if encoding.as_slice() == ["largesets"] {
                    return None; // None is ignored by filter_map
                }
                Some((path.to_string(), encoding)) // Return the tuple for the 'result'. unpacked by filter_map
            })
            .collect::<Vec<(String, Vec<String>)>>()),
        Ok(metadata) => Err(format!(
            "Path exists but not a directory: {:?} metadata: {:?}",
            path, metadata
        )),
        Err(err) => Err(format!(
            "Cannot find large datasets at {:?} error: {}",
            path, err
        )),
    }
}
