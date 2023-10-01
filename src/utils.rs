#![allow(dead_code)]

use crate::assets::LANGUAGES;
use crate::consts::{
    ENCODING_MARKS, IANA_SUPPORTED, IANA_SUPPORTED_SIMILAR, RE_POSSIBLE_ENCODING_INDICATION,
    UNICODE_RANGES_COMBINED, UNICODE_SECONDARY_RANGE_KEYWORD, UTF8_MAXIMAL_ALLOCATION,
};
use crate::entity::Language;
use ahash::{HashSet, HashSetExt};
use cached::proc_macro::cached;
use cached::UnboundCache;
use encoding::label::encoding_from_whatwg_label;
use encoding::{CodecError, DecoderTrap, EncoderTrap, Encoding, EncodingRef, StringWriter};
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use unic::char::property::EnumeratedCharProperty;
use unic::ucd::normal::decompose_canonical;
use unic::ucd::{GeneralCategory, Name};

// Utils module

// check if character category contains one of categories_partial or
// if character category is exactly one of categories_exact or
// character is from range which has name, contains one of ranges_partial
fn in_category(
    character: &char,
    categories_exact: &[&str],
    categories_partial: &[&str],
    ranges_partial: &[&str],
) -> bool {
    // unicode category part
    let category = GeneralCategory::of(*character).abbr_name();
    if categories_exact.contains(&category)
        || categories_partial.iter().any(|&cp| category.contains(cp))
    {
        return true;
    }
    // unicode range part
    if !ranges_partial.is_empty() {
        if let Some(range) = unicode_range(character) {
            return ranges_partial.iter().any(|&r| range.contains(r));
        }
    }
    false
}

// check if character description contains at least one of patterns
fn in_description(character: &char, patterns: &[&str]) -> bool {
    Name::of(*character)
        .map(|description| {
            patterns
                .iter()
                .any(|&s| description.to_string().contains(s))
        })
        .unwrap_or(false)
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_punctuation(character: &char) -> bool {
    in_category(character, &[], &["P"], &["Punctuation"])
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_symbol(character: &char) -> bool {
    in_category(character, &[], &["N", "S"], &["Forms"])
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_emoticon(character: &char) -> bool {
    in_category(character, &[], &[], &["Emoticons"])
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_separator(character: &char) -> bool {
    if character.is_whitespace() || ['｜', '+', '<', '>'].contains(character) {
        return true;
    }
    in_category(character, &["Po", "Pd", "Pc"], &["Z"], &[])
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_unprintable(character: &char) -> bool {
    !character.is_whitespace()
        && !character.is_ascii_graphic()
        && !['\x1A', '\u{FEFF}'].contains(character)
        && in_category(character, &["Cc"], &[], &["Control character"])
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_accentuated(character: &char) -> bool {
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

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_latin(character: &char) -> bool {
    let patterns = ["LATIN"];
    in_description(character, &patterns)
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_cjk(character: &char) -> bool {
    let patterns = ["CJK"];
    in_description(character, &patterns)
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_hiragana(character: &char) -> bool {
    let patterns = ["HIRAGANA"];
    in_description(character, &patterns)
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_katakana(character: &char) -> bool {
    let patterns = ["KATAKANA"];
    in_description(character, &patterns)
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_hangul(character: &char) -> bool {
    let patterns = ["HANGUL"];
    in_description(character, &patterns)
}

#[cached(
    type = "UnboundCache<char, bool>",
    create = "{ UnboundCache::with_capacity(*UTF8_MAXIMAL_ALLOCATION) }",
    convert = r#"{ *character }"#
)]
pub(crate) fn is_thai(character: &char) -> bool {
    let patterns = ["THAI"];
    in_description(character, &patterns)
}

pub(crate) fn is_case_variable(character: &char) -> bool {
    character.is_lowercase() != character.is_uppercase()
}

pub(crate) fn is_unicode_range_secondary(range_name: &str) -> bool {
    UNICODE_SECONDARY_RANGE_KEYWORD
        .iter()
        .any(|&s| range_name.contains(s))
}

// Retrieve the Unicode range official name from a single character
pub(crate) fn unicode_range(character: &char) -> Option<&'static str> {
    let char_code = *character as u32;
    UNICODE_RANGES_COMBINED
        .iter()
        .find(|&(_, range)| range.contains(&char_code))
        .map(|(name, _)| *name)
}

pub(crate) fn range_scan(decoded_sequence: &str) -> HashSet<String> {
    let (lower, upper) = decoded_sequence.chars().size_hint();
    let mut result: HashSet<String> = HashSet::with_capacity(upper.unwrap_or(lower));
    result.extend(
        decoded_sequence
            .chars()
            .filter_map(|ch| unicode_range(&ch).map(|r| r.to_string())),
    );
    result // decoded_sequence.chars().filter_map(|ch| unicode_range(&ch).map(|r| r.to_string())).collect()
}

pub(crate) fn remove_accent(ch: &char) -> char {
    let mut base_char = None;
    decompose_canonical(*ch, |c| {
        base_char.get_or_insert(c);
    });
    base_char.map_or(*ch, |c| c)
}

pub(crate) fn should_strip_sig_or_bom(_iana_encoding: &str) -> bool {
    // it looks like we always remove it in Rust (but in Python version no)
    true
}

// Verify is a specific encoding is a multi byte one based on it IANA name
pub fn is_multi_byte_encoding(name: &str) -> bool {
    [
        "utf-8",
        "utf-16le",
        "utf-16be",
        "euc-jp",
        "euc-kr",
        "iso-2022-jp",
        "gbk",
        "gb18030",
        "hz",
        "big5",
        "shift_jis",
    ]
    .contains(&name)
}

// Try to detect multibyte encoding by signature
pub(crate) fn identify_sig_or_bom(sequence: &[u8]) -> (Option<String>, Option<&[u8]>) {
    ENCODING_MARKS
        .iter()
        .find(|&(_, enc_sig)| sequence.starts_with(enc_sig))
        .map(|(enc_name, enc_sig)| (Some(enc_name.to_string()), Some(*enc_sig)))
        .unwrap_or((None, None))
}

// Try to get standard name by alternative labels
pub fn iana_name(cp_name: &str) -> Option<&str> {
    IANA_SUPPORTED
        .contains(&cp_name) // first just try to search it in our list
        .then(|| cp_name)
        .or_else(|| {
            // if not found, try to use alternative way
            encoding_from_whatwg_label(cp_name).map(|enc| enc.whatwg_name().unwrap_or(enc.name()))
        })
}

pub(crate) fn is_cp_similar(iana_name_a: &str, iana_name_b: &str) -> bool {
    IANA_SUPPORTED_SIMILAR.contains_key(iana_name_a)
        && IANA_SUPPORTED_SIMILAR[iana_name_a].contains(&iana_name_b)
}

// Extract using ASCII-only decoder any specified encoding in the first n-bytes.
pub(crate) fn any_specified_encoding(sequence: &[u8], search_zone: usize) -> Option<String> {
    encoding::all::ASCII
        .decode(
            &sequence[0..search_zone.min(sequence.len())],
            DecoderTrap::Ignore,
        )
        .ok()
        .and_then(|test_string| {
            RE_POSSIBLE_ENCODING_INDICATION
                .captures_iter(&test_string)
                .map(|c| c.extract())
                .filter_map(|(_, [specified_encoding])| iana_name(specified_encoding))
                .next()
                .map(|found_iana| found_iana.to_string())
        })
}

// Calculate similarity of two single byte encodings
pub(crate) fn cp_similarity(iana_name_a: &str, iana_name_b: &str) -> f32 {
    // we don't want to compare multi-byte encodings
    if is_multi_byte_encoding(iana_name_a) || is_multi_byte_encoding(iana_name_b) {
        return 0.0;
    }

    if let (Some(encoder_a), Some(encoder_b)) = (
        encoding_from_whatwg_label(iana_name_a),
        encoding_from_whatwg_label(iana_name_b),
    ) {
        let character_match_count = (1..255u8)
            .filter(|&ch| {
                let res_a = encoder_a.decode(&[ch], DecoderTrap::Ignore).ok();
                let res_b = encoder_b.decode(&[ch], DecoderTrap::Ignore).ok();
                res_a.is_some() && res_a == res_b //check that they aren't none and equal
            })
            .count();
        return character_match_count as f32 / 254.0;
    }
    0.0 // Return 0.0 if encoders could not be retrieved.
}

// Test Decoding bytes to string with specified encoding without writing result to memory
// returns true if everything is correctly decoded, otherwise false
struct DecodeTestResult {
    only_test: bool,
    data: String,
}
impl StringWriter for DecodeTestResult {
    fn writer_hint(&mut self, expectedlen: usize) {
        if self.only_test {
            return;
        }
        let newlen = self.data.len() + expectedlen;
        self.data.reserve(newlen);
    }
    fn write_char(&mut self, c: char) {
        if self.only_test {
            return;
        }
        self.data.push(c);
    }
    fn write_str(&mut self, s: &str) {
        if self.only_test {
            return;
        }
        self.data.push_str(s);
    }
}
impl DecodeTestResult {
    pub fn get_buffer(&self) -> &str {
        &self.data
    }
}

// Decode bytes to string with specified encoding
// if is_chunk = true it will try to fix first and end bytes for multibyte encodings
pub fn decode(
    input: &[u8],
    from_encoding: &str,
    how_process_errors: DecoderTrap,
    only_test: bool,
    is_chunk: bool,
) -> Result<String, String> {
    let encoder = encoding_from_whatwg_label(from_encoding)
        .ok_or(format!("Encoding '{}' not found", from_encoding))?;

    let mut buf = DecodeTestResult {
        only_test,
        data: String::new(),
    };
    let mut err = CodecError {
        upto: 0,
        cause: Cow::from(String::new()),
    };
    let chunk_len = input.len();
    let mut begin_offset: usize = 0;
    let mut end_offset: usize = chunk_len;
    let mut error_occured: bool;
    loop {
        let res = decode_to(
            encoder,
            &input[begin_offset..end_offset],
            how_process_errors,
            &mut buf,
        );
        error_occured = res.is_err();
        if let DecoderTrap::Strict = how_process_errors {
            if !is_chunk || res.is_ok() || !is_multi_byte_encoding(from_encoding) {
                break;
            }
            err = res.unwrap_err();
            if err.cause.contains("invalid sequence") {
                begin_offset += 1;
            } else if err.cause.contains("incomplete sequence") {
                end_offset -= 1;
            }
            if end_offset - begin_offset < 1 || begin_offset > 3 || (chunk_len - end_offset) > 3 {
                break;
            }
        } else {
            break;
        }
    }
    if error_occured {
        return Err(format!("{} at index {}", err.cause, err.upto));
    }
    Ok(String::from(buf.get_buffer()))
}

// Copied implementation of decode_to from encoder lib
// (we need index of problematic chars & hacks for chunks)
fn decode_to(
    encoder: EncodingRef,
    input: &[u8],
    trap: DecoderTrap,
    ret: &mut dyn StringWriter,
) -> Result<(), CodecError> {
    let mut decoder = encoder.raw_decoder();
    let mut remaining = 0;
    loop {
        let (offset, err) = decoder.raw_feed(&input[remaining..], ret);
        let unprocessed = remaining + offset;

        match err {
            Some(err) => {
                remaining = remaining.wrapping_add_signed(err.upto);
                if !trap.trap(&mut *decoder, &input[unprocessed..remaining], ret) {
                    return Err(err);
                }
            }
            None => {
                remaining = input.len();
                if let Some(err) = decoder.raw_finish(ret) {
                    remaining = remaining.wrapping_add_signed(err.upto);
                    if !trap.trap(&mut *decoder, &input[unprocessed..remaining], ret) {
                        return Err(err);
                    }
                }
                if remaining >= input.len() {
                    return Ok(());
                }
            }
        }
    }
}

// Encode string to vec of bytes with specified encoding
pub fn encode(
    input: &str,
    to_encoding: &str,
    how_process_errors: EncoderTrap,
) -> Result<Vec<u8>, String> {
    if let Some(encoder) = encoding_from_whatwg_label(to_encoding) {
        return Ok(encoder.encode(input, how_process_errors)?);
    }
    Err(format!("Encoding '{}' not found", to_encoding))
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
            [range_a, range_b].iter().any(|&x| x == "Basic Latin"),  // is_any_basic_latin
        ) {
            (true, true, _, _, _, _) => return false, // both are japanese
            //either is japanese and either contains CJK
            (true, _, true, _, _, _) | (_, true, true, _, _, _) => return false,
            // either has both CJK and Hanguls
            (_, _, true, true, _, _) => return false,
            // either has chinese and dedicated punctuation and separators
            (_, _, true, _, true, _) => return false,
            // either has hangul and basic latin
            (_, _, _, true, _, true) => return false,
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

// Get large datasets
fn collect_large_sets(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
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
pub fn get_large_test_datasets() -> Result<Vec<(String, Vec<String>)>, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/data/largesets/");

    match fs::metadata(&path) {
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
                if encoding.len() == 1 && encoding.first()? == "largesets" {
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
