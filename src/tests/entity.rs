use crate::entity::{CharsetMatch, CharsetMatches, CoherenceMatch, Language};

#[test]
fn test_charset_matches() {
    ///////////////////////////////////////////////////////////////////////////////////////////
    // CharsetMatches tests
    ///////////////////////////////////////////////////////////////////////////////////////////

    let mut c_matches = CharsetMatches::new(Some(vec![CharsetMatch::new(
        &[0xD0, 0xA2, 0xD0, 0xB5, 0xD1, 0x81, 0xD1, 0x82],
        "utf-8",
        0.01,
        false,
        &vec![
            CoherenceMatch {
                language: &Language::Russian,
                score: 0.99,
            },
            CoherenceMatch {
                language: &Language::Ukrainian,
                score: 0.8,
            },
        ],
        None,
    )]));
    assert_eq!(c_matches.len(), 1);

    // append new CharsetMatch
    c_matches.append(CharsetMatch::new(
        &[0xD0, 0xA2, 0xD0, 0xB5, 0xD1, 0x81, 0xD1, 0x82],
        "utf-16le",
        0.011,
        false,
        &vec![
            CoherenceMatch {
                language: &Language::Russian,
                score: 0.95,
            },
            CoherenceMatch {
                language: &Language::Kazakh,
                score: 0.7,
            },
        ],
        None,
    ));
    assert_eq!(c_matches.len(), 2);

    // check best match
    assert!(c_matches.get_best().is_some());
    assert_eq!(c_matches.get_best().unwrap().encoding(), "utf-8");

    // check get by encoding
    assert!(c_matches.get_by_encoding("utf-8").is_some());
    assert_eq!(
        c_matches
            .get_by_encoding("utf-8")
            .unwrap()
            .decoded_payload()
            .unwrap(),
        "Тест"
    );

    // test indexation impl
    assert_eq!(c_matches[0].encoding(), "utf-8");

    // test iteration
    let mut i = 0;
    for _ in c_matches.iter_mut() {
        i += 1;
    }
    assert_eq!(i, 2);
    let mut i = 0;
    for _ in c_matches.iter_mut() {
        i += 1;
    }
    assert_eq!(i, 2);
    assert_eq!(c_matches.len(), 2);

    ///////////////////////////////////////////////////////////////////////////////////////////
    // CharsetMatch tests
    ///////////////////////////////////////////////////////////////////////////////////////////

    // PartialEq test
    assert_ne!(c_matches[0], c_matches[1]);
    assert_eq!(
        c_matches[1],
        CharsetMatch::new(
            &[0xD0, 0xA2, 0xD0, 0xB5, 0xD1, 0x81, 0xD1, 0x82],
            "utf-16le",
            0.044,
            true,
            &vec!(
                CoherenceMatch {
                    language: &Language::Russian,
                    score: 0.1,
                },
                CoherenceMatch {
                    language: &Language::Kazakh,
                    score: 0.5,
                },
            ),
            None,
        )
    );

    // most_probably_language
    assert_eq!(c_matches[0].most_probably_language(), &Language::Russian);

    // languages
    assert!(c_matches[0].languages().contains(&&Language::Ukrainian));

    // multi_byte_usage
    for m in c_matches.iter_mut() {
        assert_eq!(m.multi_byte_usage(), 0.5);
    }

    // chaos_percents
    assert_eq!(c_matches[0].chaos_percents(), 1.0);
    assert_eq!(c_matches[1].chaos_percents(), 1.1);

    // coherence_percents
    assert_eq!(c_matches[0].coherence_percents(), 99.0);
    assert_eq!(c_matches[1].coherence_percents(), 95.0);

    // unicode_ranges
    for m in c_matches.iter_mut() {
        if m.encoding() == "utf-8" {
            assert!(m.unicode_ranges().contains(&String::from("Cyrillic")));
        } else {
            assert!(m
                .unicode_ranges()
                .contains(&String::from("CJK Unified Ideographs")));
        }
    }

    // encoding_aliases
    assert!(c_matches[0].encoding_aliases().contains(&"unicode11utf8"));
}
