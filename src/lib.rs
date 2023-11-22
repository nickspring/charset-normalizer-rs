//! charset-normalizer-rs
//! ======================
//!
//! The Real First Universal Charset Detector, Rust version.
//! Motivated by original Python version of `charset-normalizer`,
//!
//! This library helps read text from an unknown charset encoding.
//! All IANA character set names for which the Rust `encoding` library provides codecs are supported.
//!
//! This project is port of original Pyhon version of [Charset Normalizer](https://github.com/Ousret/charset_normalizer).
//! The biggest difference between Python and Rust versions - number of supported encodings as each langauge has own encoding / decoding library.
//! In Rust version only encoding from [WhatWG standard](https://encoding.spec.whatwg.org) are supported.
//! Python version supports more encodings, but a lot of them are old almost unused ones.
//!
//! # Performance:
//!
//! This library in comparison to Python version is more faster (2-3 times faster, than MYPYC version of charset-normalizer, 4-6 times faster, than usual Python version).
//! All measurements are approximated.
//!
//! # Library:
//!
//! Library offers two main methods:
//!
//! * `from_bytes` processes text using bytes as input parameter
//! * `from_path` processes text using filename as input parameter
//!
//! ## Examples:
//!
//! ```rust
//! use charset_normalizer_rs::from_bytes;
//!
//! fn test_from_bytes() {
//!     let result = from_bytes(&vec![0x84, 0x31, 0x95, 0x33], None);
//!     let best_guess = result.get_best();
//!     assert_eq!(
//!         best_guess.unwrap().encoding(),
//!         "gb18030",
//!     );
//! }
//! test_from_bytes();
//! ```
//!
//! ```rust
//! use std::path::Path;
//! use charset_normalizer_rs::from_path;
//!
//! fn test_from_path() {
//!     let result = from_path(Path::new("src/tests/data/samples/sample-chinese.txt"), None).unwrap();
//!     let best_guess = result.get_best();
//!     assert_eq!(
//!         best_guess.unwrap().encoding(),
//!         "big5",
//!     );
//! }
//! test_from_path();
//! ```
//!
//! # CLI tool:
//!
//! Binary CLI tool is included within this package. It has similar to Python version input parameters and output data.
//!
//! ## Installation:
//!
//! ```shell
//! cargo install charset-normalizer-rs
//! ```
//!
//! ## Usage:
//!
//! ```shell
//! normalizer -h
//!
//! usage: normalizer [-h] [-v] [-a] [-n] [-m] [-r] [-f] [-t THRESHOLD] [--version] files [files ...]
//!
//! The Real First Universal Charset Detector. Discover originating encoding used on text file. Normalize text to unicode.
//!
//! positional arguments:
//!   files                 File(s) to be analysed
//!
//! options:
//!   -h, --help            show this help message and exit
//!   -v, --verbose         Display complementary information about file if any. Stdout will contain logs about the detection process.
//!   -a, --with-alternative
//!                         Output complementary possibilities if any. Top-level JSON WILL be a list.
//!   -n, --normalize       Permit to normalize input file. If not set, program does not write anything.
//!   -m, --minimal         Only output the charset detected to STDOUT. Disabling JSON output.
//!   -r, --replace         Replace file when trying to normalize it instead of creating a new one.
//!   -f, --force           Replace file without asking if you are sure, use this flag with caution.
//!   -t THRESHOLD, --threshold THRESHOLD
//!                         Define a custom maximum amount of chaos allowed in decoded content. 0. <= chaos <= 1.
//!   --version             Show version information and exit.
//! ```
//!
//! ## Example:
//!
//! ```shell
//! normalizer src/tests/data/samples/sample-chinese.txt
//! ```
//!
//! This will produce such JSON output:
//!
//! ```json
//! {
//!     "path": ".../src/tests/data/samples/sample-chinese.txt",
//!     "encoding": "big5",
//!     "encoding_aliases": [
//!         "big5_tw",
//!         "csbig5",
//!         "x_mac_trad_chinese"
//!     ],
//!     "alternative_encodings": [
//!         "big5hkscs",
//!         "cp950"
//!     ],
//!     "language": "Chinese",
//!     "alphabets": [
//!         "Basic Latin",
//!         "CJK Compatibility Forms",
//!         "CJK Symbols and Punctuation",
//!         "CJK Unified Ideographs",
//!         "Control character",
//!         "Halfwidth and Fullwidth Forms"
//!     ],
//!     "has_sig_or_bom": false,
//!     "chaos": 0.0,
//!     "coherence": 12.21,
//!     "unicode_path": null,
//!     "is_preferred": true
//! }
//! ```
use crate::cd::{
    coherence_ratio, encoding_languages, mb_encoding_languages, merge_coherence_ratios,
};
use crate::consts::{IANA_SUPPORTED, MAX_PROCESSED_BYTES, TOO_BIG_SEQUENCE, TOO_SMALL_SEQUENCE};
use crate::entity::{CharsetMatch, CharsetMatches, CoherenceMatches, NormalizerSettings};
use crate::md::mess_ratio;
use crate::utils::{
    any_specified_encoding, decode, iana_name, identify_sig_or_bom, is_cp_similar,
    is_invalid_chunk, is_multi_byte_encoding,
};
use encoding::DecoderTrap;
use log::{debug, trace};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub mod assets;
// TODO: Revisit float conversions when we want to push for accuracy
#[allow(clippy::cast_lossless, clippy::cast_precision_loss)]
mod cd;
pub mod consts;
pub mod entity;
mod md;
mod tests;
pub mod utils;

// Given a raw bytes sequence, return the best possibles charset usable to render str objects.
// If there is no results, it is a strong indicator that the source is binary/not text.
// By default, the process will extract 5 blocks of 512o each to assess the mess and coherence of a given sequence.
// And will give up a particular code page after 20% of measured mess. Those criteria are customizable at will.
//
// The preemptive behavior DOES NOT replace the traditional detection workflow, it prioritize a particular code page
// but never take it for granted. Can improve the performance.
//
// You may want to focus your attention to some code page or/and not others, use cp_isolation and cp_exclusion for that
// purpose.
//
// This function will strip the SIG in the payload/sequence every time except on UTF-16, UTF-32.
// By default the library does not setup any handler other than the NullHandler, if you choose to set the 'explain'
// toggle to True it will alter the logger configuration to add a StreamHandler that is suitable for debugging.
// Custom logging format and handler can be set manually.
pub fn from_bytes(bytes: &[u8], settings: Option<NormalizerSettings>) -> CharsetMatches {
    // init settings with default values if it's None and recheck include_encodings and
    // exclude_encodings settings
    let mut settings = settings.unwrap_or_default();
    if !settings.include_encodings.is_empty() {
        settings.include_encodings = settings
            .include_encodings
            .iter()
            .map(|e| iana_name(e).unwrap().to_string())
            .collect();
        trace!(
            "include_encodings is set. Use this flag for debugging purpose. \
        Limited list of encoding allowed : {}.",
            settings.include_encodings.join(", ")
        );
    }
    if !settings.exclude_encodings.is_empty() {
        settings.exclude_encodings = settings
            .exclude_encodings
            .iter()
            .map(|e| iana_name(e).unwrap().to_string())
            .collect();
        trace!(
            "exclude_encodings is set. Use this flag for debugging purpose. \
        Limited list of encoding allowed : {}.",
            settings.exclude_encodings.join(", ")
        );
    }

    // check for empty
    let bytes_length = bytes.len();
    if bytes_length == 0 {
        debug!("Encoding detection on empty bytes, assuming utf_8 intention.");
        return CharsetMatches::from_single(CharsetMatch::default());
    }

    // check min length
    if bytes_length <= (settings.chunk_size * settings.steps) {
        trace!(
            "override steps ({}) and chunk_size ({}) as content does not \
            fit ({} byte(s) given) parameters.",
            settings.steps,
            settings.chunk_size,
            bytes_length,
        );
        settings.steps = 1;
        settings.chunk_size = bytes_length;
    }

    if settings.steps > 1 && bytes_length / settings.steps < settings.chunk_size {
        settings.chunk_size = bytes_length / settings.steps;
    }

    // too small length
    if bytes_length < TOO_SMALL_SEQUENCE {
        trace!(
            "Trying to detect encoding from a tiny portion of ({}) byte(s).",
            bytes_length
        );
    }

    // too big length
    let is_too_large_sequence = bytes_length > TOO_BIG_SEQUENCE;
    if is_too_large_sequence {
        trace!(
            "Using lazy str decoding because the payload is quite large, ({}) byte(s).",
            bytes_length
        );
    }

    // start to build prioritized encodings array
    let mut prioritized_encodings: Vec<&str> = vec![];

    // search for encoding in the content
    let mut specified_encoding: String = String::new();
    if settings.preemptive_behaviour {
        if let Some(enc) = any_specified_encoding(bytes, 4096) {
            trace!(
                "Detected declarative mark in sequence. Priority +1 given for {}.",
                &enc
            );
            specified_encoding = enc.to_string();
            prioritized_encodings.push(&specified_encoding);
        }
    }

    // check bom & sig
    let (sig_encoding, sig_payload) = identify_sig_or_bom(bytes);
    if let (Some(sig_enc), Some(sig_pay)) = (&sig_encoding, sig_payload) {
        trace!(
            "Detected a SIG or BOM mark on first {} byte(s). Priority +1 given for {}.",
            sig_pay.len(),
            sig_enc,
        );
        prioritized_encodings.push(sig_enc);
    }

    // add ascii & utf-8
    prioritized_encodings.extend(&["ascii", "utf-8"]);

    // generate array of encodings for probing with prioritizing
    let mut iana_encodings: VecDeque<&str> = VecDeque::from(IANA_SUPPORTED.clone());
    for pe in prioritized_encodings.iter().rev() {
        if let Some(index) = iana_encodings.iter().position(|x| x == pe) {
            let value = iana_encodings.remove(index).unwrap();
            iana_encodings.push_front(value);
        }
    }

    // Main processing loop variables
    let mut tested_but_hard_failure: Vec<&str> = vec![];
    let mut tested_but_soft_failure: Vec<&str> = vec![];
    let mut fallback_ascii: Option<CharsetMatch> = None;
    let mut fallback_u8: Option<CharsetMatch> = None;
    let mut fallback_specified: Option<CharsetMatch> = None;
    let mut results: CharsetMatches = CharsetMatches::default();

    // Iterate and probe our encodings
    'iana_encodings_loop: for encoding_iana in iana_encodings {
        if (!settings.include_encodings.is_empty()
            && !settings
                .include_encodings
                .contains(&encoding_iana.to_string()))
            || settings
                .exclude_encodings
                .contains(&encoding_iana.to_string())
        {
            continue;
        }
        let bom_or_sig_available: bool = sig_encoding.as_deref() == Some(encoding_iana);
        // let strip_sig_or_bom = true // unlike python version this is always true in rust
        let is_multi_byte_decoder: bool = is_multi_byte_encoding(encoding_iana);

        // utf-16le & utf-16be cannot be identified without BOM
        if !bom_or_sig_available && ["utf-16le", "utf-16be"].contains(&encoding_iana) {
            trace!(
                "Encoding {} won't be tested as-is because it require a BOM. Will try some sub-encoder LE/BE",
                encoding_iana,
            );
            continue;
        }

        // fast pre-check
        let start_idx = match bom_or_sig_available {
            true => sig_payload.unwrap().len(),
            false => 0,
        };
        let end_idx = match is_too_large_sequence && !is_multi_byte_decoder {
            true => MAX_PROCESSED_BYTES,
            false => bytes_length,
        };
        let decoded_payload: Option<String> = if let Ok(payload) = decode(
            &bytes[start_idx..end_idx],
            encoding_iana,
            DecoderTrap::Strict,
            is_too_large_sequence && !is_multi_byte_decoder,
            false,
        ) {
            (!is_too_large_sequence || is_multi_byte_decoder).then_some(payload)
        } else {
            trace!(
                "Code page {} does not fit given bytes sequence at ALL.",
                encoding_iana,
            );
            tested_but_hard_failure.push(encoding_iana);
            continue 'iana_encodings_loop;
        };

        // soft failed pre-check
        // important thing! it occurs sometimes fail detection
        for encoding_soft_failed in &tested_but_soft_failure {
            if is_cp_similar(encoding_iana, encoding_soft_failed) {
                trace!("{} is deemed too similar to code page {} and was consider unsuited already. Continuing!",
                    encoding_iana,
                    encoding_soft_failed,
                );
                continue 'iana_encodings_loop;
            }
        }

        // lets split input by chunks and try to parse them
        let max_chunk_gave_up = 2.max(settings.steps / 4);
        let mut early_stop_count: usize = 0;
        let mut lazy_str_hard_failure = false;
        let mut md_ratios: Vec<f32> = vec![];

        // detect target languages
        let target_languages = if is_multi_byte_decoder {
            mb_encoding_languages(encoding_iana)
        } else {
            encoding_languages(encoding_iana.to_string())
        };
        trace!(
            "{} should target any language(s) of {:?}",
            encoding_iana,
            target_languages,
        );

        // main loop over chunks in our input
        // we go over bytes or chars - it depends on previous code
        let seq_len = match &decoded_payload {
            Some(payload) => payload.chars().count(),
            None => bytes_length,
        };
        let starting_offset = match (bom_or_sig_available, &decoded_payload) {
            (true, None) => start_idx,
            _ => 0,
        };
        let offsets = (starting_offset..seq_len).step_by((seq_len / settings.steps).max(1));

        // Chunks Loop
        // Iterate over chunks of bytes or chars
        let mut md_chunks: Vec<String> = vec![];
        'chunks_loop: for offset in offsets {
            let decoded_chunk_result = match &decoded_payload {
                // Chars processing
                Some(payload) => Ok(payload
                    .chars()
                    .skip(offset)
                    .take(settings.chunk_size)
                    .collect()),
                // Bytes processing
                None => decode(
                    &bytes[offset..(offset + settings.chunk_size).min(seq_len)],
                    encoding_iana,
                    DecoderTrap::Strict,
                    false,
                    false,
                ),
            };

            if is_invalid_chunk(&decoded_chunk_result, encoding_iana) {
                trace!(
                    "LazyStr Loading: After MD chunk decode, code page {} \
                    does not fit given bytes sequence at ALL. {}",
                    encoding_iana,
                    match decoded_chunk_result {
                        Ok(_) => String::from("non-ascii"),
                        Err(message) => message.to_string(),
                    },
                );
                early_stop_count = max_chunk_gave_up;
                lazy_str_hard_failure = true;
                break 'chunks_loop;
            }
            let decoded_chunk = decoded_chunk_result.unwrap();

            // MD ratios calc
            md_chunks.push(decoded_chunk.clone());
            md_ratios.push(mess_ratio(decoded_chunk, Some(settings.threshold)));
            if md_ratios.last().unwrap() >= &settings.threshold {
                early_stop_count += 1;
            }
            if early_stop_count >= max_chunk_gave_up {
                break 'chunks_loop;
            }
        }

        // We might want to check the remainder of sequence
        // Only if initial MD tests passes
        if !lazy_str_hard_failure && is_too_large_sequence && !is_multi_byte_decoder {
            let decoded_chunk_result = decode(
                &bytes[MAX_PROCESSED_BYTES..],
                encoding_iana,
                DecoderTrap::Strict,
                false,
                false,
            );
            if is_invalid_chunk(&decoded_chunk_result, encoding_iana) {
                trace!(
                    "LazyStr Loading: After final lookup, code page {} does not fit \
                    given bytes sequence at ALL. {}",
                    encoding_iana,
                    decoded_chunk_result.unwrap_err().to_string(),
                );
                tested_but_hard_failure.push(encoding_iana);
                continue 'iana_encodings_loop;
            }
        }

        // process mean mess ratio
        let mean_mess_ratio = match md_ratios.is_empty() {
            true => 0.0,
            false => md_ratios.iter().sum::<f32>() / (md_ratios.len() as f32),
        };

        if mean_mess_ratio >= *settings.threshold || early_stop_count >= max_chunk_gave_up {
            tested_but_soft_failure.push(encoding_iana);
            trace!(
                "{} was excluded because of initial chaos probing. \
                Gave up {} time(s). Computed mean chaos is {} %.",
                encoding_iana,
                early_stop_count,
                mean_mess_ratio * 100.0,
            );
            // Preparing those fallbacks in case we got nothing.
            if settings.enable_fallback
                && !lazy_str_hard_failure
                && prioritized_encodings.contains(&encoding_iana)
            {
                let fallback_entry = Some(CharsetMatch::new(
                    bytes,
                    encoding_iana,
                    f32::from(settings.threshold),
                    false,
                    &vec![],
                    decoded_payload.as_deref(),
                ));

                match encoding_iana {
                    e if e == specified_encoding => fallback_specified = fallback_entry,
                    "ascii" => fallback_ascii = fallback_entry,
                    _ => fallback_u8 = fallback_entry,
                }
            }
            continue 'iana_encodings_loop;
        }
        trace!(
            "{} passed initial chaos probing. Mean measured chaos is {} %",
            encoding_iana,
            mean_mess_ratio * 100.0,
        );

        // CD rations calc
        // We shall skip the CD when its about ASCII
        // Most of the time its not relevant to run "language-detection" on it.
        let mut cd_ratios: Vec<CoherenceMatches> = vec![];
        if encoding_iana != "ascii" {
            cd_ratios.extend(md_chunks.iter().filter_map(|chunk| {
                coherence_ratio(
                    chunk.clone(),
                    Some(settings.language_threshold),
                    Some(target_languages.clone()),
                )
                .ok()
            }));
        }

        // process cd ratios
        let cd_ratios_merged = merge_coherence_ratios(&cd_ratios);
        if !cd_ratios_merged.is_empty() {
            trace!(
                "We detected language {:?} using {}",
                cd_ratios_merged,
                encoding_iana
            );
        }

        // process results
        results.append(CharsetMatch::new(
            bytes,
            encoding_iana,
            mean_mess_ratio,
            bom_or_sig_available,
            &cd_ratios_merged,
            decoded_payload.as_deref(),
        ));

        if (mean_mess_ratio < 0.1 && prioritized_encodings.contains(&encoding_iana))
            || encoding_iana == sig_encoding.clone().unwrap_or_default()
        {
            debug!(
                "Encoding detection: {} is most likely the one.",
                encoding_iana
            );
            return CharsetMatches::from_single(
                results.get_by_encoding(encoding_iana).unwrap().clone(),
            );
        }
    }

    // fallbacks
    if results.is_empty() {
        let fb = match (&fallback_specified, &fallback_u8, &fallback_ascii) {
            (Some(specified), _, _) => Some(specified),
            (None, Some(u8_fallback), None) => Some(u8_fallback),
            (None, Some(u8_fallback), Some(ascii))
                if u8_fallback.decoded_payload() != ascii.decoded_payload() =>
            {
                Some(u8_fallback)
            }
            (None, _, Some(ascii)) => Some(ascii),
            _ => None,
        };
        if let Some(fb_to_pass) = fb {
            debug!(
                "Encoding detection: will be used as a fallback match {}",
                fb_to_pass.encoding()
            );
            results.append(fb_to_pass.clone());
        };
    }

    // final logger information
    if results.is_empty() {
        debug!("Encoding detection: Unable to determine any suitable charset.");
    } else {
        debug!(
            "Encoding detection: Found {} as plausible (best-candidate) for content. \
            With {} alternatives.",
            results.get_best().unwrap().encoding(),
            results.len() - 1,
        );
    }
    results
}

// Same thing than the function from_bytes but with one extra step.
// Opening and reading given file path in binary mode.
// Can return Error.
pub fn from_path(
    path: &Path,
    settings: Option<NormalizerSettings>,
) -> Result<CharsetMatches, String> {
    // read file
    let mut file = File::open(path).map_err(|e| format!("Error opening file: {e}"))?;
    let file_size = file.metadata().map(|m| m.len()).unwrap_or_default();

    let mut buffer = Vec::with_capacity(file_size as usize);
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Error reading from file: {e}"))?;

    // calculate
    Ok(from_bytes(&buffer, settings))
}
