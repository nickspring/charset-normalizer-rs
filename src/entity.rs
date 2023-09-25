#![allow(unused_variables)]

use crate::cd::{encoding_languages, mb_encoding_languages};
use crate::consts::{IANA_SUPPORTED_ALIASES, TOO_BIG_SEQUENCE};
use crate::utils::{decode, iana_name, is_multi_byte_encoding, range_scan, round_float};
use clap::Parser;
use encoding::DecoderTrap;
use ordered_float::OrderedFloat;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::Index;
use std::path::PathBuf;
use std::time::Duration;

/////////////////////////////////////////////////////////////////////////////////////
// Languages
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    German,
    French,
    Dutch,
    Italian,
    Polish,
    Spanish,
    Russian,
    Japanese,
    Portuguese,
    Swedish,
    Chinese,
    Ukrainian,
    Norwegian,
    Finnish,
    Vietnamese,
    Czech,
    Hungarian,
    Korean,
    Indonesian,
    Turkish,
    Romanian,
    Farsi,
    Arabic,
    Danish,
    Serbian,
    Lithuanian,
    Slovene,
    Slovak,
    Hebrew,
    Bulgarian,
    Croatian,
    Hindi,
    Estonian,
    Thai,
    Greek,
    Tamil,
    Kazakh,
    Unknown,
}

impl Display for Language {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/////////////////////////////////////////////////////////////////////////////////////
// CoherenceMatch & CoherenceMatches
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Clone)]
pub struct CoherenceMatch {
    pub language: &'static Language,
    pub score: f32,
}

pub type CoherenceMatches = Vec<CoherenceMatch>;

/////////////////////////////////////////////////////////////////////////////////////
// CharsetMatch
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct CharsetMatch {
    payload: Vec<u8>,
    encoding: String,

    mean_mess_ratio: f32,
    coherence_matches: CoherenceMatches,

    has_sig_or_bom: bool,
    fingerprint: String,

    submatch: Vec<CharsetMatch>,
    decoded_payload: Option<String>,
}

impl Display for CharsetMatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({})", self.payload, self.encoding)
    }
}

impl Debug for CharsetMatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({})", self.payload, self.encoding)
    }
}

impl PartialEq<Self> for CharsetMatch {
    fn eq(&self, other: &Self) -> bool {
        self.encoding == other.encoding && self.fingerprint == other.fingerprint
    }
}

impl PartialOrd<Self> for CharsetMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mess_difference = (self.mean_mess_ratio - other.mean_mess_ratio).abs();
        let coherence_a = self.coherence();
        let coherence_b = other.coherence();
        let coherence_difference = (coherence_a - coherence_b).abs();

        // Below 1% difference --> Use Coherence
        if mess_difference < 0.01 && coherence_difference > 0.02 {
            // When having a tough decision, use the result that decoded as many multi-byte as possible.
            if mess_difference == 0.0 && coherence_difference == 0.0 {
                return other
                    .multi_byte_usage()
                    .partial_cmp(&self.multi_byte_usage());
            }
            return coherence_b.partial_cmp(&coherence_a);
        }
        self.mean_mess_ratio.partial_cmp(&other.mean_mess_ratio)
    }
}

impl CharsetMatch {
    // Init function
    pub fn new(
        payload: &[u8],
        encoding: &str,
        mean_mess_ratio: f32,
        has_sig_or_bom: bool,
        coherence_matches: &CoherenceMatches,
        decoded_payload: Option<&str>,
    ) -> Self {
        let mut obj = CharsetMatch {
            payload: Vec::from(payload),
            encoding: String::from(encoding),
            mean_mess_ratio,
            coherence_matches: coherence_matches.clone(),
            has_sig_or_bom,
            submatch: vec![],
            decoded_payload: decoded_payload.map(String::from),
            fingerprint: String::new(),
        };

        // decoded payload recalc
        if obj.decoded_payload.is_none() {
            if let Ok(res) = decode(
                &obj.payload,
                obj.encoding.as_str(),
                DecoderTrap::Strict,
                false,
                true,
            ) {
                obj.decoded_payload =
                    Some(res.strip_prefix('\u{feff}').unwrap_or(&res).to_string());
            }
        }
        if obj.decoded_payload.is_some() {
            obj.fingerprint = format!(
                "{:?}",
                blake3::hash(
                    obj.decoded_payload
                        .as_ref()
                        .unwrap_or(&String::default())
                        .as_bytes()
                )
            );
        }
        obj
    }
    // Add submatch
    pub fn add_submatch(&mut self, submatch: CharsetMatch) {
        self.submatch.push(submatch.clone());
        //self.decoded_payload = None;
    }

    // Alphabets
    pub fn alphabets(&self) -> Vec<String> {
        todo!();
    }

    // Get encoding aliases according to https://encoding.spec.whatwg.org/encodings.json
    pub fn encoding_aliases(&self) -> Vec<&'static str> {
        if let Some(res) = IANA_SUPPORTED_ALIASES.get(&self.encoding.as_str()) {
            return res.clone();
        }
        vec![]
    }
    pub fn bom(&self) -> bool {
        self.has_sig_or_bom
    }
    pub fn byte_order_mark(&self) -> bool {
        self.has_sig_or_bom
    }
    pub fn encoding(&self) -> &str {
        &self.encoding
    }
    pub fn chaos(&self) -> f32 {
        self.mean_mess_ratio
    }
    // Most probable language found in decoded sequence. If none were detected or inferred, the property will return
    // Language::Unknown
    pub fn most_probably_language(&self) -> &'static Language {
        if self.coherence_matches.is_empty() {
            // Trying to infer the language based on the given encoding
            // Its either English or we should not pronounce ourselves in certain cases.
            if self.suitable_encodings().contains(&String::from("ascii")) {
                return &Language::English;
            }

            let languages = if is_multi_byte_encoding(&self.encoding) {
                mb_encoding_languages(&self.encoding)
            } else {
                encoding_languages(self.encoding.clone())
            };

            if languages.is_empty() || languages.contains(&&Language::Unknown) {
                return &Language::Unknown;
            }

            return languages.first().unwrap();
        }
        self.coherence_matches
            .first()
            .map(|lang| lang.language)
            .unwrap()
    }
    // Return the complete list of possible languages found in decoded sequence.
    // Usually not really useful. Returned list may be empty even if 'language' property return something != 'Unknown'.
    pub fn languages(&self) -> Vec<&'static Language> {
        self.coherence_matches
            .iter()
            .map(|cm| cm.language)
            .collect()
    }
    // Has submatch
    pub fn has_submatch(&self) -> bool {
        !self.submatch.is_empty()
    }
    // Return submatch list
    pub fn submatch(&self) -> &Vec<CharsetMatch> {
        &self.submatch
    }
    // Multibyte usage ratio
    pub fn multi_byte_usage(&self) -> f32 {
        let decoded_chars = self.decoded_payload().unwrap_or_default().chars().count() as f32;
        let payload_len = self.payload.len() as f32;

        1.0 - (decoded_chars / payload_len)
    }
    // Original untouched bytes
    pub fn raw(&self) -> &Vec<u8> {
        &self.payload
    }
    // Return chaos in percents with rounding
    pub fn chaos_percents(&self) -> f32 {
        round_float(self.chaos() * 100.0, 3)
    }
    // Return coherence in percents with rounding
    pub fn coherence_percents(&self) -> f32 {
        round_float(self.coherence() * 100.0, 3)
    }
    // Most relevant language coherence
    pub fn coherence(&self) -> f32 {
        if self.coherence_matches.is_empty() {
            return 0.0;
        }
        self.coherence_matches
            .first()
            .map(|lang| lang.score)
            .unwrap()
    }

    // To recalc decoded_payload field
    pub fn decoded_payload(&self) -> Option<&str> {
        self.decoded_payload.as_deref()
    }

    // Retrieve the unique blake3 hash computed using the transformed (re-encoded) payload.
    // Not the original one. Original Python version has sha256 algorithm
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    // The complete list of encodings that output the exact SAME str result and therefore could be the originating
    // encoding. This list does include the encoding available in property 'encoding'.
    pub fn suitable_encodings(&self) -> Vec<String> {
        let mut result: Vec<String> = self.submatch.iter().map(|s| s.encoding.clone()).collect();
        result.insert(0, self.encoding.clone());
        result
    }
    // Returns sorted list of unicode ranges (if exists)
    pub fn unicode_ranges(&self) -> Vec<String> {
        let mut ranges: Vec<String> = range_scan(self.decoded_payload().unwrap_or_default())
            .iter()
            .cloned()
            .collect();
        ranges.sort_unstable();
        ranges
    }
}

/////////////////////////////////////////////////////////////////////////////////////
// CharsetMatches
// Container with every CharsetMatch items ordered by default from most probable
// to the less one.
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct CharsetMatches {
    items: Vec<CharsetMatch>,
}

pub struct CharsetMatchesIterMut<'a> {
    items: std::slice::IterMut<'a, CharsetMatch>,
}

pub struct CharsetMatchesIter<'a> {
    items: std::slice::Iter<'a, CharsetMatch>,
}

impl CharsetMatches {
    // Initialization method
    pub fn new(items: Option<Vec<CharsetMatch>>) -> Self {
        let mut items = items.unwrap_or_default();
        CharsetMatches::resort(&mut items);
        CharsetMatches { items }
    }
    // Insert a single match. Will be inserted accordingly to preserve sort.
    // Can be inserted as a submatch.
    pub fn append(&mut self, item: CharsetMatch) {
        // We should disable the submatch factoring when the input file is too heavy
        // (conserve RAM usage)
        if item.payload.len() <= *TOO_BIG_SEQUENCE {
            for m in self.items.iter_mut() {
                if m.fingerprint() == item.fingerprint()
                    && m.mean_mess_ratio == item.mean_mess_ratio
                {
                    m.add_submatch(item.clone());
                    return;
                }
            }
        }
        self.items.push(item);
        CharsetMatches::resort(&mut self.items);
    }
    // Simply return the first match. Strict equivalent to matches[0].
    pub fn get_best(&self) -> Option<&CharsetMatch> {
        if self.items.is_empty() {
            return None;
        }
        self.items.first()
    }
    // Retrieve a single item either by its position or encoding name (alias may be used here).
    pub fn get_by_encoding(&self, encoding: &str) -> Option<&CharsetMatch> {
        let encoding = iana_name(encoding)?;
        self.items
            .iter()
            .find(|&i| i.suitable_encodings().contains(&encoding.to_string()))
    }
    // Resort items by relevancy (for internal use)
    fn resort(items: &mut [CharsetMatch]) {
        items.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    }
    // iterator
    pub fn iter_mut(&mut self) -> CharsetMatchesIterMut {
        CharsetMatchesIterMut {
            items: self.items.iter_mut(),
        }
    }
    pub fn iter(&self) -> CharsetMatchesIter {
        CharsetMatchesIter {
            items: self.items.iter(),
        }
    }
    // len
    pub fn len(&self) -> usize {
        self.items.len()
    }
    // is empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Index<usize> for CharsetMatches {
    type Output = CharsetMatch;
    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index]
    }
}

impl<'a> Iterator for CharsetMatchesIterMut<'a> {
    type Item = &'a mut CharsetMatch;

    fn next(&mut self) -> Option<Self::Item> {
        self.items.next()
    }
}

impl<'a> Iterator for CharsetMatchesIter<'a> {
    type Item = &'a CharsetMatch;

    fn next(&mut self) -> Option<Self::Item> {
        self.items.next()
    }
}

#[derive(Clone)]
pub struct NormalizerSettings {
    /// How many steps (chunks) should be used from file
    pub steps: usize,
    /// Each chunk size
    pub chunk_size: usize,
    /// Mess ration threshold
    pub threshold: OrderedFloat<f32>,
    /// Specify probing encodings exactly
    pub include_encodings: Vec<String>,
    /// Exclude these encodings from probing
    pub exclude_encodings: Vec<String>,
    /// Allow try to find charset in the text
    pub preemptive_behaviour: bool,
    /// Language detector threshold
    pub language_threshold: OrderedFloat<f32>,
    /// Allow fallback to ASCII / UTF-8
    pub enable_fallback: bool,
}

impl Default for NormalizerSettings {
    fn default() -> Self {
        NormalizerSettings {
            steps: 5,
            chunk_size: 512,
            threshold: OrderedFloat(0.2),
            include_encodings: vec![],
            exclude_encodings: vec![],
            preemptive_behaviour: true,
            language_threshold: OrderedFloat(0.1),
            enable_fallback: true,
        }
    }
}

/////////////////////////////////////////////////////////////////////////////////////
// Performance binary application
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Debug)]
#[command(name = "Performance check for charset-normalizer-rs vs chardet vs chardetng")]
#[command(author, version, about, long_about = None)]
pub struct PerformanceArgs {
    /// Apply artificial size increase to challenge the detection mechanism further
    #[arg(short, long, default_value_t = 1)]
    pub size_increase: u8,
}

// Struct to save result of each test in performance app
pub struct PerformanceResult {
    /// Performance test duration
    pub duration: Duration,
    /// Is result accurate?
    pub correct: bool,
}

/////////////////////////////////////////////////////////////////////////////////////
// Normalizer CLI application
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Parser, Debug)]
#[command(
    name = "The Real First Universal Charset Detector. Discover originating encoding used on text file. Normalize text to unicode."
)]
#[command(author, version, about, long_about = None)]
pub struct CLINormalizerArgs {
    /// File(s) to be analysed
    #[arg(required = true, action = clap::ArgAction::Append)]
    pub files: Vec<PathBuf>,

    /// Display complementary information about file if any. Stdout will contain logs about the detection process.
    #[arg(short = 'v', long = "verbose", default_value_t = false)]
    pub verbose: bool,

    /// Output complementary possibilities if any. Top-level JSON WILL be a list.
    #[arg(short = 'a', long = "with-alternative", default_value_t = false)]
    pub alternatives: bool,

    /// Permit to normalize input file. If not set, program does not write anything.
    #[arg(short, long, default_value_t = false)]
    pub normalize: bool,

    /// Only output the charset detected to STDOUT. Disabling JSON output.
    #[arg(short, long, default_value_t = false)]
    pub minimal: bool,

    /// Replace file when trying to normalize it instead of creating a new one.
    #[arg(short, long, default_value_t = false)]
    pub replace: bool,

    /// Replace file without asking if you are sure, use this flag with caution.
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Define a custom maximum amount of chaos allowed in decoded content. 0. <= chaos <= 1.
    #[arg(short, long, default_value_t = 0.2)]
    pub threshold: f32,
}

#[derive(Default, Debug, Serialize)]
pub struct CLINormalizerResult {
    /// Path to analysed file
    pub path: PathBuf,
    /// Guessed encoding
    pub encoding: Option<String>,
    /// Possible aliases of guessed encoding
    pub encoding_aliases: Vec<String>,
    /// Alternative possible encodings
    pub alternative_encodings: Vec<String>,
    /// Most probably language
    pub language: String,
    /// Found alphabets
    pub alphabets: Vec<String>,
    /// Does it has SIG or BOM mark?
    pub has_sig_or_bom: bool,
    /// Chaos (mess) level
    pub chaos: f32,
    /// Coherence (language detection) level
    pub coherence: f32,
    /// Path to decoded data
    pub unicode_path: Option<PathBuf>,
    pub is_preferred: bool,
}
