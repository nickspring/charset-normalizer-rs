#![allow(unused_variables)]
use crate::assets::*;
use crate::consts::TOO_SMALL_SEQUENCE;
use crate::entity::*;
use crate::utils::*;
use cache_macro_stable_rust::cache;
use counter::Counter;
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;
use lru_cache::LruCache;
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use strsim::jaro;

//
// Coherence detection module
//

// Return associated unicode ranges in a single byte code page.
pub(crate) fn encoding_unicode_range(iana_name: &str) -> Result<Vec<&str>, String> {
    if is_multi_byte_encoding(iana_name) {
        return Err(String::from(
            "Function not supported on multi-byte code page",
        ));
    }
    let mut result: HashMap<&str, u32> = HashMap::new();
    let mut character_count: u32 = 0;

    if let Some(p) = encoding_from_whatwg_label(iana_name) {
        for i in 0x40..0xFF {
            if let Ok(chunk) = p.decode(&[i], DecoderTrap::Ignore) {
                if let Some(first_char) = chunk.chars().next() {
                    if let Some(range) = unicode_range(&first_char) {
                        if is_unicode_range_secondary(range.to_string()) {
                            continue;
                        }
                        let range_count = result.entry(range).or_insert(0);
                        *range_count += 1;
                        character_count += 1;
                    }
                }
            }
        }
    } else {
        return Err(String::from("No decoder found for this encoding"));
    }
    let mut result: Vec<&str> = result
        .iter()
        .filter(|(&name, &value)| (value as f32 / character_count as f32) >= 0.15)
        .map(|(&name, &value)| name)
        .collect();
    result.sort();
    Ok(result)
}

// Return inferred languages used with a unicode range.
pub(crate) fn unicode_range_languages(primary_range: &str) -> Vec<&'static Language> {
    if primary_range.is_empty() {
        return vec![];
    }
    LANGUAGES
        .iter()
        .filter_map(|(language, characters, _, _)| {
            characters
                .chars()
                .find(|&character| unicode_range(&character).unwrap_or_default() == primary_range)
                .map(|_| language)
        })
        .collect()
}

// Single-byte encoding language association.
// Some code page are heavily linked to particular language(s).
// This function does the correspondence.
#[cache(LruCache : LruCache::new(128))]
pub(crate) fn encoding_languages(iana_name: String) -> Vec<&'static Language> {
    let unicode_ranges = encoding_unicode_range(&iana_name).unwrap_or_default();
    let mut primary_range: Option<&str> = None;

    for specified_range in unicode_ranges {
        if !specified_range.contains("Latin") {
            primary_range = Some(specified_range);
            break;
        }
    }

    if primary_range.is_none() {
        return vec![&Language::Unknown];
    }

    unicode_range_languages(primary_range.unwrap())
}

// Multi-byte encoding language association. Some code page are heavily linked to particular language(s).
// This function does the correspondence.
pub(crate) fn mb_encoding_languages(iana_name: &str) -> Vec<&'static Language> {
    let mut result = vec![];
    if let Some(found) = ENCODING_TO_LANGUAGE.get(iana_name) {
        result.push(found);
    }
    result
}

// Return associated languages associated to given characters
#[allow(clippy::ptr_arg)]
pub(crate) fn alphabet_languages(
    characters: &Vec<&char>,
    ignore_non_latin: bool,
) -> Vec<&'static Language> {
    let mut languages: Vec<(&Language, f32)> = vec![];
    let source_characters_set: HashSet<_> = characters.iter().cloned().copied().collect();
    let source_has_accents = source_characters_set.iter().any(is_accentuated);

    for (language, language_characters, target_have_accents, target_pure_latin) in LANGUAGES.iter()
    {
        if (ignore_non_latin && !*target_pure_latin)
            || (!*target_have_accents && source_has_accents)
        {
            continue;
        }

        let language_characters_set: HashSet<_> = language_characters.chars().collect();
        let intersection: HashSet<_> = language_characters_set
            .intersection(&source_characters_set)
            .cloned()
            .collect();

        let ratio: f32 = intersection.len() as f32 / language_characters_set.len() as f32;
        if ratio >= 0.2 {
            languages.push((language, ratio));
        }
    }
    // reverse sort
    languages.sort_by(|&a, &b| b.1.partial_cmp(&a.1).unwrap());
    languages.iter().map(|&lang| lang.0).collect()
}

// Given a decoded text sequence, return a list of str. Unicode range / alphabet separation.
// Ex. a text containing English/Latin with a bit a Hebrew will return two items in the resulting list;
// One containing the latin letters and the other hebrew.
pub(crate) fn alpha_unicode_split(decoded_sequence: &str) -> Vec<String> {
    let mut layers: HashMap<&str, String> = HashMap::new();

    for ch in decoded_sequence.chars() {
        if !ch.is_alphabetic() {
            continue;
        }
        if let Some(character_range) = unicode_range(&ch) {
            let mut layer_target_range: Option<&str> = None;
            for discovered_range in layers.keys() {
                if !is_suspiciously_successive_range(Some(discovered_range), Some(character_range))
                {
                    layer_target_range = Some(discovered_range);
                    break;
                }
            }
            if layer_target_range.is_none() {
                layer_target_range = Some(character_range);
            }

            let layer = layers
                .entry(layer_target_range.unwrap())
                .or_insert(String::from(""));
            *layer += &ch.to_lowercase().to_string();
        }
    }
    layers.values().cloned().collect()
}

// Determine if a ordered characters list (by occurrence from most appearance to rarest) match a particular language.
// The result is a ratio between 0. (absolutely no correspondence) and 1. (near perfect fit).
// Beware that is function is not strict on the match in order to ease the detection. (Meaning close match is 1.)
// Remark: ordered_characters is string here, with chars ordered by popularity.
// Original function in Python was more complicated and slower
pub(crate) fn characters_popularity_compare(
    language: &Language,
    ordered_characters: &str,
) -> Result<f32, String> {
    let language_data = get_language_data(language)?;
    Ok(jaro(ordered_characters, language_data.0) as f32)
}

// We shall NOT return more than one "English" in CoherenceMatches because it is an alternative
// of "English" (the same for Japan language). This function only keeps the best match.
pub(crate) fn filter_alt_coherence_matches(results: &CoherenceMatches) -> CoherenceMatches {
    let mut index: HashMap<&Language, f32> = HashMap::new();
    for result in results {
        let score = index.entry(result.language).or_insert(0.0);
        *score = result.score.max(*score);
    }
    index
        .into_iter()
        .map(|(language, score)| CoherenceMatch { language, score })
        .collect()
}

// This function merge results previously given by the function coherence_ratio.
// The return type is the same as coherence_ratio.
pub(crate) fn merge_coherence_ratios(results: &Vec<CoherenceMatches>) -> CoherenceMatches {
    let mut index: HashMap<&Language, Vec<f32>> = HashMap::new();

    for result in results {
        for sub_result in result {
            let score = index.entry(sub_result.language).or_insert(vec![]);
            score.push(sub_result.score);
        }
    }

    let mut merge: Vec<CoherenceMatch> = index
        .iter()
        .map(|(&lang, scores)| CoherenceMatch {
            language: lang,
            score: round_float(scores.iter().sum::<f32>() / (scores.len() as f32), 4),
        })
        .collect();

    merge.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    merge
}

// The main function. Detect ANY language that can be identified in given sequence.
// The sequence will be analysed by layers.
// A layer = Character extraction by alphabets/ranges.
#[cache(LruCache: LruCache::new(2048))]
pub(crate) fn coherence_ratio(
    decoded_sequence: String,
    threshold: Option<OrderedFloat<f32>>,
    include_languages: Option<Vec<&'static Language>>,
) -> Result<CoherenceMatches, String> {
    let threshold = f32::from(threshold.unwrap_or(OrderedFloat(0.1)));
    let mut include_languages = include_languages.unwrap_or_default();
    let ignore_non_latin =
        include_languages.len() == 1 && include_languages.first() == Some(&&Language::Unknown);
    if ignore_non_latin {
        include_languages.clear();
    }

    let mut results: CoherenceMatches = vec![];
    let mut sufficient_match_count: u64 = 0;

    for layer in alpha_unicode_split(&decoded_sequence) {
        if layer.chars().count() <= *TOO_SMALL_SEQUENCE {
            continue;
        }
        let most_common = layer.chars().collect::<Counter<_>>().most_common_ordered();
        let popular_character_ordered: Vec<&char> = most_common.iter().map(|(ch, _)| ch).collect();

        let languages = if include_languages.is_empty() {
            alphabet_languages(&popular_character_ordered, ignore_non_latin)
        } else {
            include_languages.clone()
        };

        let popular_character_ordered_as_string: String =
            popular_character_ordered.iter().copied().collect();

        // Convert the String into a &str
        for language in languages {
            let ratio: f32 =
                characters_popularity_compare(language, &popular_character_ordered_as_string)?;

            if ratio < threshold {
                continue;
            } else if ratio >= 0.8 {
                sufficient_match_count += 1;
            }

            results.push(CoherenceMatch {
                language,
                score: round_float(ratio, 4),
            });

            if sufficient_match_count >= 3 {
                break;
            }
        }
    }
    results = filter_alt_coherence_matches(&results);
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(results)
}
