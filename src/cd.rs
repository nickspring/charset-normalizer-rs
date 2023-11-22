#![allow(unused_variables)]
use crate::assets::{ENCODING_TO_LANGUAGE, LANGUAGES, LANGUAGE_SUPPORTED_COUNT};
use crate::consts::TOO_SMALL_SEQUENCE;
use crate::entity::{CoherenceMatch, CoherenceMatches, Language};
use crate::utils::{
    get_language_data, is_accentuated, is_multi_byte_encoding, is_suspiciously_successive_range,
    is_unicode_range_secondary, unicode_range,
};
use ahash::{HashMap, HashMapExt, HashSet};
use cached::proc_macro::cached;
use counter::Counter;
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;
use ordered_float::OrderedFloat;
use strsim::jaro;

//
// Coherence detection module
//

// Return associated unicode ranges in a single byte code page.
pub(crate) fn encoding_unicode_range(iana_name: &str) -> Result<Vec<&str>, String> {
    if is_multi_byte_encoding(iana_name) {
        return Err("Function not supported on multi-byte code page".to_string());
    }
    let encoder = encoding_from_whatwg_label(iana_name)
        .ok_or("No decoder found for this encoding".to_string())?;

    let byte_range = 0x40..0xFF; // utf8 range. range.len()==191
    let mut result: HashMap<&str, u8> = HashMap::with_capacity(byte_range.len());

    byte_range.for_each(|i| {
        if let Some(range) = encoder
            .decode(&[i], DecoderTrap::Ignore)
            .ok()
            .and_then(|chunk| chunk.chars().next())
            .and_then(unicode_range)
            .filter(|&range| !is_unicode_range_secondary(range))
        {
            *result.entry(range).or_insert(0) += 1;
        }
    });
    let character_count: u8 = result.values().sum();
    let threshold = 0.15;
    let mut result: Vec<&str> = result
        .iter()
        .filter(|(_, &value)| (value as f32 / character_count as f32) >= threshold)
        .map(|(&name, _)| name)
        .collect();
    result.sort_unstable();
    Ok(result)
}

// Return inferred languages used with a unicode range.
pub(crate) fn unicode_range_languages(primary_range: &str) -> Vec<&'static Language> {
    LANGUAGES
        .iter()
        .filter_map(|(language, characters, _, _)| {
            characters
                .chars()
                .find(|char| unicode_range(*char).unwrap_or_default() == primary_range)
                .map(|_| language)
        })
        .collect::<Vec<&Language>>()
}

// Single-byte encoding language association.
// Some code page are heavily linked to particular language(s).
// This function does the correspondence.
#[cached(size = 128)]
pub(crate) fn encoding_languages(iana_name: String) -> Vec<&'static Language> {
    match encoding_unicode_range(&iana_name)
        .unwrap_or_default()
        .iter()
        .find(|&&range| !range.contains("Latin"))
    {
        Some(&range) => unicode_range_languages(range),
        None => vec![&Language::Unknown],
    }
}

// Multi-byte encoding language association. Some code page are heavily linked to particular language(s).
// This function does the correspondence.
pub(crate) fn mb_encoding_languages(iana_name: &str) -> Vec<&'static Language> {
    ENCODING_TO_LANGUAGE
        .get(iana_name)
        .map_or(vec![], |found| vec![found])
}

// Return associated languages associated to given characters
#[allow(clippy::ptr_arg)]
pub(crate) fn alphabet_languages(
    characters: &[char],
    ignore_non_latin: bool,
) -> Vec<&'static Language> {
    let mut languages: Vec<(&Language, f32)> = Vec::with_capacity(*LANGUAGE_SUPPORTED_COUNT);
    let source_characters_set: HashSet<char> = characters.iter().copied().collect();
    let source_has_accents = source_characters_set
        .iter()
        .any(|&char| is_accentuated(char));

    for (language, language_characters, target_have_accents, target_pure_latin) in LANGUAGES.iter()
    {
        if (ignore_non_latin && !target_pure_latin) || (!target_have_accents && source_has_accents)
        {
            continue;
        }

        let language_characters_set: HashSet<char> = language_characters.chars().collect();
        let intersection: HashSet<char> = language_characters_set
            .intersection(&source_characters_set)
            .copied()
            .collect();

        let ratio: f32 = intersection.len() as f32 / language_characters_set.len() as f32;
        if ratio >= 0.2 {
            languages.push((language, ratio));
        }
    }
    // reverse sort
    languages.sort_unstable_by(|&a, &b| b.1.partial_cmp(&a.1).unwrap());
    languages.iter().map(|&lang| lang.0).collect()
}

// Given a decoded text sequence, return a list of str. Unicode range / alphabet separation.
// Ex. a text containing English/Latin with a bit a Hebrew will return two items in the resulting list;
// One containing the latin letters and the other hebrew.
pub(crate) fn alpha_unicode_split(decoded_sequence: &str) -> Vec<String> {
    let mut layers: HashMap<&str, String> = HashMap::new();

    for ch in decoded_sequence.chars().filter(|c| c.is_alphabetic()) {
        if let Some(character_range) = unicode_range(ch) {
            let layer_key: &str = layers
                .keys()
                .find(|key| !is_suspiciously_successive_range(Some(key), Some(character_range)))
                .copied()
                .unwrap_or(character_range);
            let layer = layers.entry(layer_key).or_default();
            layer.extend(ch.to_lowercase());
        }
    }
    layers.into_values().collect()
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
    let mut index: HashMap<&Language, f32> = HashMap::with_capacity(results.len());
    for result in results {
        let score = index.entry(result.language).or_default();
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
    let mut index: HashMap<&Language, Vec<f32>> = HashMap::with_capacity(results.len());
    results
        .iter()
        .flatten()
        .for_each(|result| index.entry(result.language).or_default().push(result.score));

    let mut merge: Vec<CoherenceMatch> = index
        .iter()
        .map(|(&lang, scores)| CoherenceMatch {
            language: lang,
            score: scores.iter().sum::<f32>() / (scores.len() as f32),
        })
        .collect();

    merge.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    merge
}

// The main function. Detect ANY language that can be identified in given sequence.
// The sequence will be analysed by layers.
// A layer = Character extraction by alphabets/ranges.
#[cached(size = 2048)]
pub(crate) fn coherence_ratio(
    decoded_sequence: String,
    threshold: Option<OrderedFloat<f32>>,
    include_languages: Option<Vec<&'static Language>>,
) -> Result<CoherenceMatches, String> {
    let threshold = f32::from(threshold.unwrap_or(OrderedFloat(0.1)));
    let mut include_languages: Vec<&Language> = include_languages.unwrap_or_default();
    let ignore_non_latin = include_languages == vec![&Language::Unknown];
    if ignore_non_latin {
        include_languages.clear();
    }

    let mut results: CoherenceMatches = vec![];
    let mut sufficient_match_count: u64 = 0;

    for layer in alpha_unicode_split(&decoded_sequence) {
        if layer.chars().count() <= TOO_SMALL_SEQUENCE {
            continue;
        }
        let most_common = layer.chars().collect::<Counter<_>>().most_common_ordered();
        let popular_character_ordered: Vec<char> = most_common.iter().map(|(ch, _)| *ch).collect();

        let languages = if include_languages.is_empty() {
            alphabet_languages(&popular_character_ordered, ignore_non_latin)
        } else {
            include_languages.clone()
        };

        let popular_character_ordered_as_string: String =
            popular_character_ordered.iter().collect();

        // Convert the String into a &str
        for language in languages {
            let ratio: f32 =
                characters_popularity_compare(language, &popular_character_ordered_as_string)?;

            match ratio {
                r if r < threshold => continue,
                r if r >= 0.8 => sufficient_match_count += 1,
                _ => {}
            }

            results.push(CoherenceMatch {
                language,
                score: ratio,
            });

            if sufficient_match_count >= 3 {
                break;
            }
        }
    }
    results = filter_alt_coherence_matches(&results);
    results.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(results)
}
