#![allow(unused_variables)]

use crate::cd::encoding_languages;
use crate::consts::TOO_BIG_SEQUENCE;
use crate::enc::{Encoding, IsChunk, WantDecode};
use crate::utils::range_scan;
use ordered_float::OrderedFloat;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::Index;

/////////////////////////////////////////////////////////////////////////////////////
// Languages
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
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
pub(crate) struct CoherenceMatch {
    pub language: &'static Language,
    pub score: OrderedFloat<f32>,
}

pub(crate) type CoherenceMatches = Vec<CoherenceMatch>;

/////////////////////////////////////////////////////////////////////////////////////
// CharsetMatch
/////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct CharsetMatch {
    encoding: &'static Encoding,
    payload_len: usize,

    mean_mess_ratio: OrderedFloat<f32>,
    coherence_matches: CoherenceMatches,

    has_sig_or_bom: bool,

    submatch: Vec<CharsetMatch>,
    decoded_payload: Option<String>,
}

impl Display for CharsetMatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({})", self.decoded_payload, self.encoding)
    }
}

impl Debug for CharsetMatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({})", self.decoded_payload, self.encoding)
    }
}

impl Default for CharsetMatch {
    fn default() -> Self {
        CharsetMatch {
            encoding: Encoding::by_name("utf-8").expect("have utf8"),
            payload_len: 0,
            mean_mess_ratio: OrderedFloat(0.0),
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

impl Eq for CharsetMatch {}

impl Ord for CharsetMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        let mess_difference = (self.mean_mess_ratio - other.mean_mess_ratio).abs();
        let coherence_a = OrderedFloat(self.coherence());
        let coherence_b = OrderedFloat(other.coherence());
        let coherence_difference = (coherence_a - coherence_b).abs();

        // Below 1% difference --> Use Coherence
        if mess_difference < 0.01 {
            if coherence_difference > 0.02 {
                return coherence_b.cmp(&coherence_a);
            }
            let multibyte_usage_a = OrderedFloat(self.multi_byte_usage());
            let multibyte_usage_b = OrderedFloat(other.multi_byte_usage());
            let multibyte_usage_delta = (multibyte_usage_a - multibyte_usage_b).abs();
            if multibyte_usage_delta > f32::EPSILON {
                return multibyte_usage_b.cmp(&multibyte_usage_a);
            }
        }
        self.mean_mess_ratio.cmp(&other.mean_mess_ratio)
    }
}

impl PartialOrd<Self> for CharsetMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl CharsetMatch {
    // Init function
    pub(crate) fn new(
        payload: &[u8],
        encoding: &'static Encoding,
        mean_mess_ratio: f32,
        has_sig_or_bom: bool,
        coherence_matches: &CoherenceMatches,
        decoded_payload: Option<&str>,
    ) -> Self {
        CharsetMatch {
            encoding,
            payload_len: payload.len(),
            mean_mess_ratio: OrderedFloat(mean_mess_ratio),
            coherence_matches: coherence_matches.clone(),
            has_sig_or_bom,
            submatch: vec![],
            decoded_payload: decoded_payload.map(String::from).or_else(|| {
                encoding
                    .decode(payload, WantDecode::Yes, IsChunk::Yes)
                    .ok()
                    .map(|res| res.strip_prefix('\u{feff}').unwrap_or(&res).to_string())
            }),
        }
    }

    // Add submatch
    pub(crate) fn add_submatch(&mut self, submatch: &CharsetMatch) {
        self.submatch.push(submatch.clone());
        //self.decoded_payload = None;
    }

    /// Get encoding aliases according to <https://encoding.spec.whatwg.org/encodings.json>
    pub fn encoding_aliases(&self) -> &'static [&'static str] {
        self.encoding.aliases()
    }

    /// Did this match have a byte order mark?
    pub fn bom(&self) -> bool {
        self.has_sig_or_bom
    }

    pub fn encoding(&self) -> &'static Encoding {
        self.encoding
    }
    pub fn chaos(&self) -> f32 {
        self.mean_mess_ratio.0
    }

    /// Most probable language found in decoded sequence. If none were detected or inferred, the property will return
    /// Language::Unknown
    pub fn most_probably_language(&self) -> &'static Language {
        self.coherence_matches.first().map_or_else(
            // Default case: Trying to infer the language based on the given encoding
            || {
                if self
                    .suitable_encodings()
                    .iter()
                    .any(|enc| enc.name() == "ascii")
                {
                    &Language::English
                } else {
                    let language = if self.encoding.is_multi_byte_encoding() {
                        self.encoding.language()
                    } else {
                        encoding_languages(self.encoding.name()).first().copied()
                    };
                    language.unwrap_or(&Language::Unknown)
                }
            },
            |lang| lang.language,
        )
    }

    /// Return the complete list of possible languages found in decoded sequence.
    /// Usually not really useful. Returned list may be empty even if 'language' property return something != 'Unknown'.
    pub fn languages(&self) -> Vec<&'static Language> {
        self.coherence_matches
            .iter()
            .map(|cm| cm.language)
            .collect()
    }

    /// Has submatch
    pub fn has_submatch(&self) -> bool {
        !self.submatch.is_empty()
    }

    /// Return submatch list
    pub fn submatch(&self) -> &Vec<CharsetMatch> {
        &self.submatch
    }

    /// Multibyte usage ratio
    pub fn multi_byte_usage(&self) -> f32 {
        let decoded_chars = self.decoded_payload().unwrap_or_default().chars().count() as f32;
        let payload_len = self.payload_len as f32;

        1.0 - (decoded_chars / payload_len)
    }

    /// Return chaos in percents with rounding
    pub fn chaos_percents(&self) -> f32 {
        self.chaos() * 100.0
    }

    /// Return coherence in percents with rounding
    pub fn coherence_percents(&self) -> f32 {
        self.coherence() * 100.0
    }

    /// Most relevant language coherence
    pub fn coherence(&self) -> f32 {
        self.coherence_matches
            .first()
            .map(|lang| lang.score.0)
            .unwrap_or_default()
    }

    /// Returns the payload decoded into a string
    pub fn decoded_payload(&self) -> Option<&str> {
        self.decoded_payload.as_deref()
    }

    /// The complete list of encodings that output the exact SAME str result and therefore could be the originating
    /// encoding. This list does include the encoding available in property 'encoding'.
    pub fn suitable_encodings(&self) -> Vec<&'static Encoding> {
        std::iter::once(self.encoding)
            .chain(self.submatch.iter().map(|s| s.encoding))
            .collect()
    }

    /// Returns sorted list of unicode ranges (if exists)
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
        if item.payload_len <= TOO_BIG_SEQUENCE {
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
        let encoding = Encoding::by_name(encoding)?;
        self.items
            .iter()
            .find(|&i| i.suitable_encodings().contains(&encoding))
    }
    // Resort items by relevancy (for internal use)
    fn resort(items: &mut [CharsetMatch]) {
        items.sort_unstable();
    }
    // iterator
    pub fn iter_mut(&mut self) -> CharsetMatchesIterMut<'_> {
        CharsetMatchesIterMut {
            items: self.items.iter_mut(),
        }
    }
    pub fn iter(&self) -> CharsetMatchesIter<'_> {
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
