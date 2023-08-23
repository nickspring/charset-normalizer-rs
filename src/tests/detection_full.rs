use crate::from_path;
use crate::tests::FILES_SAMPLES;
use crate::utils::{get_large_test_datasets, is_multi_byte_encoding};
use std::path::PathBuf;

#[test]
fn test_elementary_detection() {
    for sample in &*FILES_SAMPLES {
        let filename = sample.0;
        let encoding = &sample.1;
        let language = sample.2;

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(format!("src/tests/data/samples/{}", filename));
        let result = from_path(&path, None);

        assert!(result.is_ok());
        let result = result.unwrap();
        let best_guess = result.get_best();
        let enc = best_guess.unwrap().encoding();
        let languages = best_guess.unwrap().languages();

        assert!(
            best_guess.is_some(),
            "Elementary detection has failed upon '{}'",
            filename
        );
        assert!(
            encoding.contains(&enc),
            "Elementary charset detection has failed upon '{}', {} NOT IN {:?}",
            filename,
            enc,
            encoding
        );
        assert!(
            languages.contains(&language),
            "Elementary language detection has failed upon '{}', {} NOT IN {:?}",
            filename,
            language,
            languages
        );
    }
}

#[test]
fn test_largesets() {
    for (path, encoding) in get_large_test_datasets().unwrap() {
        let result = from_path(&PathBuf::from(path.clone()), None);
        assert!(result.is_ok());

        let result = result.unwrap();
        let best_guess = result.get_best();
        let mut guess_encoding = "None";
        if best_guess.is_some() {
            guess_encoding = best_guess.unwrap().encoding();
        }
        let fail = !encoding.contains(&guess_encoding.to_string())
            && (guess_encoding == "None"
                || encoding
                    .iter()
                    .any(|x| is_multi_byte_encoding(guess_encoding) != is_multi_byte_encoding(x)));

        assert!(!fail, "Problems with {}", path);
    }
}
