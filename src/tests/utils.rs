use crate::entity::NormalizerSettings;
use crate::tests::FILES_SAMPLES;
use crate::utils::*;
use encoding::DecoderTrap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[test]
fn test_is_unicode_range_secondary() {
    assert!(!is_unicode_range_secondary("Something"));
    assert!(is_unicode_range_secondary("Extended"));
}

#[test]
fn test_unicode_range() {
    for _ in 1..10 {
        let tests = [
            ('a', "Basic Latin"),
            ('я', "Cyrillic"),
            ('ย', "Thai"),
            ('↓', "Arrows"),
            ('∅', "Mathematical Operators"),
            ('ͽ', "Greek and Coptic"),
        ];
        for test in &tests {
            assert_eq!(unicode_range(test.0), Some(test.1));
        }
    }
}

#[test]
fn test_is_ascii() {
    let tests = [
        ('a', true),
        ('я', false),
        ('ย', false),
        ('↓', false),
        ('7', true),
    ];
    for test in &tests {
        assert_eq!(test.0.is_ascii(), test.1);
    }
}

#[test]
fn test_remove_accent() {
    let tests = [('á', 'a'), ('É', 'E'), ('Ǔ', 'U'), ('↓', '↓')];
    for test in &tests {
        assert_eq!(remove_accent(test.0), test.1);
    }
}

#[test]
fn test_range_scan() {
    let test = "aÁ[!Я";
    let res = range_scan(test);
    assert_eq!(res.len(), 3);
    assert!(res.contains("Basic Latin"));
    assert!(res.contains("Latin-1 Supplement"));
    assert!(res.contains("Cyrillic"));
}

#[test]
fn test_is_multi_byte_encoding() {
    let tests = [("utf-8", true), ("ascii", false), ("euc-jp", true)];
    for test in &tests {
        assert_eq!(is_multi_byte_encoding(test.0), test.1);
    }
}

#[test]
fn test_identify_sig_or_bom() {
    let tests = [
        (
            b"\xef\xbb\xbf lol kek".as_slice(),
            Some("utf-8".to_string()),
        ),
        (b"lol kek".as_slice(), None),
    ];
    for test in &tests {
        assert_eq!(identify_sig_or_bom(test.0).0, test.1);
    }
}

#[test]
fn test_iana_name() {
    let tests = [
        ("utf8", Some("utf-8")),
        ("csibm866", Some("ibm866")),
        ("whatever", None),
        ("korean", Some("euc-kr")),
    ];
    for test in &tests {
        assert_eq!(iana_name(test.0), test.1);
    }
}

#[test]
fn test_is_cp_similar() {
    let tests = [
        ("iso-8859-14", "windows-1254", true),
        ("iso-8859-14", "euc-kr", false),
    ];
    for test in &tests {
        assert_eq!(is_cp_similar(test.0, test.1), test.2);
    }
}

#[test]
fn test_any_specified_encoding() {
    let tests =     [
        (b"<head><meta charset=\"utf8\"".as_slice(), Some("utf-8".to_string())),
        (b"<head coding='korean'> blah".as_slice(), Some("euc-kr".to_string())),
        (
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x01,\x00\x00\x00\xaf\x08\x06\x00\x00\x00G.\xe3\xb7\x00\x00\x1c\xfdIDATx".as_slice(),
            None,
        ),
        (b"<?xml version=\"1.0\" encoding=\"EUC-JP\"?>", Some("euc-jp".to_string())),
        (b"<html><head><meta charset=\"utf-8\"></head></html>", Some("utf-8".to_string())),
        (b"<html><head><meta charset=\"utf-57\"></head></html>", None),
        (b"# coding: utf-8", Some("utf-8".to_string())),
        (b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>", Some("utf-8".to_string())),
        (b"<?xml version=\"1.0\" encoding=\"US-ASCII\"?>", Some("windows-1252".to_string())),
        (b"<html><head><meta charset=WINDOWS-1252></head></html>", Some("windows-1252".to_string())),
        (b"<html><head><meta charset=\"WINDOWS-1256\"></head></html>", Some("windows-1256".to_string())),
    ];
    for test in &tests {
        assert_eq!(any_specified_encoding(test.0, 4096), test.1);
    }
}

#[test]
fn test_cp_similarity() {
    let tests = [
        ("iso-8859-14", "windows-1254", 0.75, 1.0), // high similarity
        ("windows-1250", "windows-1253", 0.5, 0.75), // low similarity
        ("iso-8859-14", "euc-kr", 0.0, 0.0),        // eur-kr is multi-byte
    ];
    for test in &tests {
        let sim = cp_similarity(test.0, test.1);
        assert!(
            sim >= test.2,
            "{} <-> {} found similarity is {}",
            test.0,
            test.1,
            sim
        );
        assert!(
            sim <= test.3,
            "{} <-> {} found similarity is {}",
            test.0,
            test.1,
            sim
        );
    }
}

#[test]
fn test_is_suspiciously_successive_range() {
    let tests = [
        (None, Some("Cyrillic"), true),
        (Some("Cyrillic"), Some("Cyrillic"), false),
        (Some("Latin"), Some("Latin Extended"), false),
        (Some("Emoticons"), Some("Latin Extended"), false),
        (
            Some("Latin"),
            Some("Combining Diacritical Marks Supplement"),
            false,
        ),
        (
            Some("Cyrillic Extended-A"),
            Some("Cyrillic Extended-B"),
            false,
        ),
        (Some("Hiragana"), Some("Katakana"), false),
        (Some("Hiragana"), Some("CJK Radicals Supplement"), false),
        (
            Some("CJK Radicals Supplement"),
            Some("Alphabetic Presentation Forms"),
            false,
        ),
        (Some("CJK Radicals Supplement"), Some("Punctuation"), false),
        (Some("Cyrillic"), Some("Basic Latin"), true),
        (Some("Cyrillic"), Some("Sundanese"), true),
    ];
    for test in &tests {
        assert_eq!(
            is_suspiciously_successive_range(test.0, test.1),
            test.2,
            "<= {:?} {:?}",
            test.0,
            test.1,
        );
    }
}

#[test]
fn test_decode_test() {
    let tests =     [
        (b"\x61\x52\x6f\x64\x20\x5a\x61\x52\x6f\x64\x20\x5a\xaa\xd8\x80\xd9\x80\xd9\x80\xd9\xb9\xd8\x80\xd9\x80\xd9\x80\xd9\x80\xd9\xaf\xd8\x8a\xd9\x80\xd9\x80\xd9\x84\xd9\xd8\x20\xd9\xa7\xd9\x84\xd9\x80\xd9\x80\xd8\x80\xd9\xaa\xd9\x80\xd9\x80\xd9\x80\xd9\x80\xd9\x88\xd9\x82\xd9\x80\xd9\x80\xd9\x8a\xd9\x80\xd9\x80\xd9\x80\xd8\x80\x20\xaa\x85\xd9\x80\xd9\x80\xd9\x80\xd9\x86\xd9\xd9\x20\xd9\x82\xd9\x80\xd8\x80\xd9\xa8\xd9\x80\xd9\x80\x00\x84".to_vec(), "euc-jp", false),
        (b"\x61\x52\x6f\x64\x20\x5a\x61\x52\x6f\x64\x20\x5a\xaa\xd8".to_vec(), "windows-1251", true),
    ];
    for test in &tests {
        let res = decode(&test.0, test.1, DecoderTrap::Strict, true, false);
        assert_eq!(res.is_ok(), test.2);
    }
}

#[test]
fn test_decode_wrong_chunks() {
    // read multibyte files, split to chunks (with non-complete sequences)
    // and decode it without fail
    // The idea is that decode function should ignore errors in the beginning and ending of chunk
    let settings = NormalizerSettings::default();
    for sample in &*FILES_SAMPLES {
        if sample.1.iter().any(|e| is_multi_byte_encoding(e)) {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push(format!("src/tests/data/samples/{}", sample.0));
            let mut file = File::open(path.to_str().unwrap()).expect("Cannot open file");
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).expect("Cannot read file");
            for chunk in buffer.chunks(settings.chunk_size) {
                let status = decode(
                    chunk,
                    sample.1.first().unwrap(),
                    DecoderTrap::Strict,
                    true,
                    true,
                );
                assert!(
                    status.is_ok(),
                    "Decode error for sample {}, {}",
                    sample.0,
                    status.unwrap_err()
                );
            }
        }
    }
}
