#![allow(unused_variables)]

use crate::cd::{encoding_languages, mb_encoding_languages};
use crate::consts::{IANA_SUPPORTED_ALIASES, TOO_BIG_SEQUENCE};
use crate::utils::{decode, iana_name, is_multi_byte_encoding, range_scan};
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

impl Default for CharsetMatch {
    fn default() -> Self {
        CharsetMatch {
            payload: vec![],
            encoding: "utf-8".to_string(),
            mean_mess_ratio: 0.0,
            coherence_matches: vec![],
            has_sig_or_bom: false,
            submatch: vec![],
            decoded_payload: None,
        }
    }
}

impl PartialEq<Self> for CharsetMatch {
    fn eq(&self, other: &Self) -> bool {
        self.encoding == other.encoding && self.decoded_payload == other.decoded_payload
    }
}

impl PartialOrd<Self> for CharsetMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mess_difference = (self.mean_mess_ratio - other.mean_mess_ratio).abs();
        let coherence_a = self.coherence();
        let coherence_b = other.coherence();
        let coherence_difference = (coherence_a - coherence_b).abs();

        // Below 1% difference --> Use Coherence
        if mess_difference < 0.01 {
            if coherence_difference > 0.02 {
                return coherence_b.partial_cmp(&coherence_a);
            }
            let multibyte_usage_a = self.multi_byte_usage();
            let multibyte_usage_b = other.multi_byte_usage();
            let multibyte_usage_delta = (multibyte_usage_a - multibyte_usage_b).abs();
            if multibyte_usage_delta > f32::EPSILON {
                return multibyte_usage_b.partial_cmp(&multibyte_usage_a);
            }
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
        CharsetMatch {
            payload: Vec::from(payload),
            encoding: String::from(encoding),
            mean_mess_ratio,
            coherence_matches: coherence_matches.clone(),
            has_sig_or_bom,
            submatch: vec![],
            decoded_payload: decoded_payload.map(String::from).or_else(|| {
                decode(payload, encoding, DecoderTrap::Strict, false, true)
                    .ok()
                    .map(|res| res.strip_prefix('\u{feff}').unwrap_or(&res).to_string())
            }),
        }
    }

    // Add submatch
    pub fn add_submatch(&mut self, submatch: &CharsetMatch) {
        self.submatch.push(submatch.clone());
        //self.decoded_payload = None;
    }

    // Get encoding aliases according to https://encoding.spec.whatwg.org/encodings.json
    pub fn encoding_aliases(&self) -> Vec<&'static str> {
        IANA_SUPPORTED_ALIASES
            .get(self.encoding.as_str())
            .cloned()
            .expect("Problem with static HashMap IANA_SUPPORTED_ALIASES")
    }
    // byte_order_mark
    pub fn bom(&self) -> bool {
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
        self.coherence_matches.first().map_or_else(
            // Default case: Trying to infer the language based on the given encoding
            || {
                if self.suitable_encodings().contains(&String::from("ascii")) {
                    &Language::English
                } else {
                    let languages = if is_multi_byte_encoding(&self.encoding) {
                        mb_encoding_languages(&self.encoding)
                    } else {
                        encoding_languages(self.encoding.clone())
                    };
                    languages.first().copied().unwrap_or(&Language::Unknown)
                }
            },
            |lang| lang.language,
        )
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
        self.chaos() * 100.0
    }
    // Return coherence in percents with rounding
    pub fn coherence_percents(&self) -> f32 {
        self.coherence() * 100.0
    }
    // Most relevant language coherence
    pub fn coherence(&self) -> f32 {
        self.coherence_matches
            .first()
            .map(|lang| lang.score)
            .unwrap_or_default()
    }

    // To recalc decoded_payload field
    pub fn decoded_payload(&self) -> Option<&str> {
        self.decoded_payload.as_deref()
    }

    // The complete list of encodings that output the exact SAME str result and therefore could be the originating
    // encoding. This list does include the encoding available in property 'encoding'.
    pub fn suitable_encodings(&self) -> Vec<String> {
        std::iter::once(self.encoding.clone())
            .chain(self.submatch.iter().map(|s| s.encoding.clone()))
            .collect()
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

#[derive(Debug, Default)]
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
    pub fn from_single(item: CharsetMatch) -> Self {
        CharsetMatches { items: vec![item] }
    }
    // Insert a single match. Will be inserted accordingly to preserve sort.
    // Can be inserted as a submatch.
    pub fn append(&mut self, item: CharsetMatch) {
        // We should disable the submatch factoring when the input file is too heavy
        // (conserve RAM usage)
        if item.payload.len() <= TOO_BIG_SEQUENCE {
            for m in &mut self.items {
                if m.decoded_payload() == item.decoded_payload()
                    && (m.mean_mess_ratio - item.mean_mess_ratio).abs() < f32::EPSILON
                {
                    m.add_submatch(&item);
                    return;
                }
            }
        }
        self.items.push(item);
        CharsetMatches::resort(&mut self.items);
    }
    // Simply return the first match. Strict equivalent to matches[0].
    pub fn get_best(&self) -> Option<&CharsetMatch> {
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
        self.items.is_empty()
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
    pub chaos: String,
    /// Coherence (language detection) level
    pub coherence: String,
    /// Path to decoded data
    pub unicode_path: Option<PathBuf>,
    pub is_preferred: bool,
}
