use charset_normalizer_rs::entity::{CLINormalizerArgs, CLINormalizerResult, NormalizerSettings};
use charset_normalizer_rs::from_path;
use clap::Parser;
use dialoguer::Confirm;
use env_logger::Env;
use ordered_float::OrderedFloat;
use std::fs::File;
use std::io::Write;
use std::{fs, process};

fn normalizer(args: &CLINormalizerArgs) -> Result<i32, String> {
    match (args.replace, args.normalize, args.force, args.threshold) {
        (true, false, _, _) => return Err("Use --replace in addition to --normalize only.".into()),
        (false, _, true, _) => return Err("Use --force in addition to --replace only.".into()),
        (_, _, _, threshold) if !(0.0..=1.0).contains(&threshold) => {
            return Err("--threshold VALUE should be between 0.0 and 1.0.".into())
        }
        _ => {}
    }

    let mut results: Vec<CLINormalizerResult> = vec![];
    let settings = NormalizerSettings {
        threshold: OrderedFloat(args.threshold),
        ..Default::default()
    };

    // go through the files
    for path in &args.files {
        let full_path = &mut fs::canonicalize(path).map_err(|err| err.to_string())?;
        let matches = from_path(full_path, Some(settings.clone()))?;
        match matches.get_best() {
            None => {
                results.push(CLINormalizerResult {
                    path: full_path.clone(),
                    language: "Unknown".to_string(),
                    chaos: format!("{:.1}", 1.0),
                    coherence: format!("{:.1}", 0.0),
                    is_preferred: true,
                    ..Default::default()
                });
                eprintln!(
                    "Unable to identify originating encoding for {:?}. {}",
                    full_path,
                    if args.threshold < 1.0 {
                        "Maybe try increasing maximum amount of chaos."
                    } else {
                        ""
                    }
                );
            }
            Some(best_guess) => {
                // add main result & alternative results
                for m in matches.iter() {
                    let normalize_result = CLINormalizerResult {
                        path: full_path.clone(),
                        encoding: Some(m.encoding().to_string()),
                        encoding_aliases: m
                            .encoding_aliases()
                            .iter()
                            .map(|s| (*s).to_string())
                            .collect(),
                        alternative_encodings: m
                            .suitable_encodings()
                            .iter()
                            .filter(|&e| e != m.encoding())
                            .cloned()
                            .collect(),
                        language: format!("{}", m.most_probably_language()),
                        alphabets: m.unicode_ranges(),
                        has_sig_or_bom: m.bom(),
                        chaos: format!("{:.1}", m.chaos_percents()),
                        coherence: format!("{:.1}", m.coherence_percents()),
                        unicode_path: None,
                        is_preferred: true,
                    };
                    if m == best_guess {
                        results.insert(0, normalize_result);
                    } else if args.alternatives {
                        results.push(normalize_result);
                    } else {
                        break;
                    }
                }

                // normalizing if need
                if args.normalize {
                    if best_guess.encoding().starts_with("utf") {
                        eprintln!(
                            "{:?} file does not need to be normalized, as it already came from unicode.",
                            full_path,
                        );
                        continue;
                    }

                    // force or confirm of replacement
                    if !args.replace {
                        let filename = full_path.file_name().unwrap().to_str().unwrap();
                        let filename = match filename.rsplit_once('.') {
                            None => format!("{}.{}", filename, best_guess.encoding()),
                            Some(split) => {
                                format!("{}.{}.{}", split.0, best_guess.encoding(), split.1)
                            }
                        };
                        full_path.set_file_name(filename);
                    } else if !args.force
                        && !Confirm::new()
                            .with_prompt(format!(
                                "Are you sure to normalize {:?} by replacing it?",
                                full_path,
                            ))
                            .interact()
                            .unwrap_or(false)
                    {
                        continue;
                    }

                    // save path to result
                    results[0].unicode_path = Some(full_path.clone());

                    // replace file contents
                    if let Err(err) = File::create(full_path).and_then(|mut file| {
                        file.write_all(best_guess.decoded_payload().unwrap().as_bytes())
                    }) {
                        return Err(err.to_string());
                    }
                }
            }
        }
    }

    // print out results
    if args.minimal {
        for path in &args.files {
            let full_path = fs::canonicalize(path).map_err(|err| err.to_string())?;
            println!(
                "{}",
                results
                    .iter()
                    .filter(|r| r.path == full_path)
                    .map(|r| r.encoding.clone().unwrap_or("undefined".to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    } else {
        println!(
            "{}",
            if results.len() > 1 {
                serde_json::to_string_pretty(&results).unwrap()
            } else {
                serde_json::to_string_pretty(&results[0]).unwrap()
            }
        );
    }
    Ok(0)
}

pub fn main() {
    let args = CLINormalizerArgs::parse();

    // verbose mode
    if args.verbose {
        env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();
    }

    // run normalizer
    match normalizer(&args) {
        Err(e) => panic!("{e}"),
        Ok(exit_code) => process::exit(exit_code),
    }
}
