use crate::cd::*;
use crate::entity::{CoherenceMatch, CoherenceMatches, Language};

#[test]
fn test_encoding_unicode_range() {
    let err_tests = [
        "utf-8",
        "big5",
        "utf-16le",             // multibyte encodings
        "non-existing-charset", // non-existing
    ];
    for test in &err_tests {
        assert!(encoding_unicode_range(test).is_err());
    }

    let ok_tests = [
        ("windows-1251", Ok(vec!["Basic Latin", "Cyrillic"])),
        ("windows-1255", Ok(vec!["Basic Latin", "Hebrew"])),
    ];
    for test in &ok_tests {
        assert_eq!(encoding_unicode_range(test.0), test.1);
    }
}

#[test]
fn test_unicode_range_languages() {
    let tests = [
        ("Cyrillic", Language::Russian),
        ("Latin Extended Additional", Language::Vietnamese),
        ("Greek and Coptic", Language::Greek),
    ];
    for (input, lang) in tests {
        let languages = unicode_range_languages(input);
        assert!(languages.contains(&&lang));
    }
}

#[test]
fn test_encoding_languages() {
    let tests = [
        ("utf-8", Language::Unknown),
        ("windows-1251", Language::Russian),
        ("windows-1255", Language::Hebrew),
    ];
    for (input, lang) in tests {
        let languages = encoding_languages(input.to_string());
        assert!(languages.contains(&&lang));
    }
}

#[test]
fn test_alphabet_languages() {
    let tests = [
        ("В низинах на восточной стороне полуострова Люнген в основном встречаются слюдяные сланцы, филлиты и доломиты. Низменности на западной стороне в основном состоят из слюдяных сланцев и небольшого количества кварцитов. За исключением ледяных шапок на вершинах Йеккеварри и Балггесварри, на полуострове Люнген преобладают каровые ледники", true, None),
        ("В низинах на восточной стороне полуострова Люнген в основном встречаются слюдяные сланцы, филлиты и доломиты. Низменности на западной стороне в основном состоят из слюдяных сланцев и небольшого количества кварцитов. За исключением ледяных шапок на вершинах Йеккеварри и Балггесварри, на полуострове Люнген преобладают каровые ледники", false, Some(Language::Russian)),
        ("Ailem ve Ben Adım Ece ve on iki yaşındayım. Her sabah 7'de uyanırım, kahvaltımı yaparım ve okula giderim. Boş zamanlarımda bahçede kitap okumayı severim. Küçük bir erkek kardeşim var. Kardeşim üç yaşında ve resim yapmayı sever. Evde her gün top oynar ve şarkı söyler. Kardeşim ve ben makarna yemeyi severiz. Bazen mutfakta yemekleri biz hazırlarız.", false, Some(Language::Turkish)),
    ];
    for (input, ignore_non_latin, expected) in tests {
        let characters: Vec<char> = input.chars().collect();
        let languages = alphabet_languages(&characters, ignore_non_latin);
        if expected.is_none() {
            assert_eq!(languages.len(), 0);
        } else {
            assert!(languages.contains(&&expected.unwrap()));
        }
    }
}

#[test]
fn test_alpha_unicode_split() {
    let tests = [
        (
            "Люнгенские Альпы (норв. Lyngsalpene, сев.‑саам. Ittuvárit, квенск. Yykeänvaarat) — горный \
            массив на северо-востоке фюльке Тромс-ог-Финнмарк в Норвегии, к востоку от города Тромсё",
            vec![
                "люнгенскиеальпынорвсевсаамквенскгорныймассивнасеверовостокефюлькетромсогфиннмарквнорвегииквостокуотгородатромсё",
                "lyngsalpeneittuvárityykeänvaarat",
            ]
        ),
    ];
    for input in tests {
        let mut layers = alpha_unicode_split(input.0);
        let mut expected = input.1.clone();
        layers.sort_unstable();
        expected.sort_unstable();
        assert_eq!(layers, expected);
    }
}

#[test]
fn test_characters_popularity_compare() {
    let tests = [
        ("оаніирвтесклудмпзяьбгйчхцї", Language::Russian, 0.8, 0.9),
        ("оаеинстрвлкмдпугяызбйьчхжц", Language::Russian, 1.0, 1.0),
    ];
    for (seq, lang, mmin, mmax) in &tests {
        let res = characters_popularity_compare(lang, seq).unwrap();
        assert!(res >= (*mmin as f32) && res <= (*mmax as f32));
    }
}

#[test]
fn test_filter_alt_coherence_matches() {
    let input: CoherenceMatches = vec![
        CoherenceMatch {
            language: &Language::English,
            score: 7.77,
        },
        CoherenceMatch {
            language: &Language::English,
            score: 4.44,
        },
    ];
    let expected_output: CoherenceMatches = vec![CoherenceMatch {
        language: &Language::English,
        score: 7.77,
    }];
    assert_eq!(filter_alt_coherence_matches(&input), expected_output);
}

#[test]
fn test_merge_coherence_ratios() {
    let input: Vec<CoherenceMatches> = vec![
        vec![
            CoherenceMatch {
                language: &Language::English,
                score: 7.77,
            },
            CoherenceMatch {
                language: &Language::English,
                score: 4.44,
            },
        ],
        vec![
            CoherenceMatch {
                language: &Language::Ukrainian,
                score: 5.0,
            },
            CoherenceMatch {
                language: &Language::Ukrainian,
                score: 10.0,
            },
        ],
        vec![CoherenceMatch {
            language: &Language::Bulgarian,
            score: 12.0,
        }],
    ];
    let mut expected_output: CoherenceMatches = vec![
        CoherenceMatch {
            language: &Language::English,
            score: 6.105,
        },
        CoherenceMatch {
            language: &Language::Ukrainian,
            score: 7.5,
        },
        CoherenceMatch {
            language: &Language::Bulgarian,
            score: 12.0,
        },
    ];
    let mut output = merge_coherence_ratios(&input);
    output.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    expected_output.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    assert_eq!(output, expected_output);
}

#[test]
fn test_coherence_ratio() {
    let tests = [
        (
            "Bсеки човек има право на образование. Oбразованието трябва да бъде безплатно, поне що се отнася до началното и основното образование.",
            vec![&Language::Bulgarian],
        ),
        (
            "Lietuviø kalba – ið baltø prokalbës kilusi lietuviø tautos kalba, kuri Lietuvoje yra valstybinë, o Europos Sàjungoje – viena ið oficialiøjø kalbø. Lietuviðkai kalba apie tris milijonus þmoniø (dauguma jø gyvena Lietuvoje). Drauge su latviø, mirusiomis prûsø, jotvingiø ir kitomis baltø kalbomis, priklauso indoeuropieèiø kalbø ðeimos baltø kalbø grupei.
            Pirmieji lietuviø kalbos raðytiniai paminklai atsirado vëlokai, apie XVI a., taèiau net dabartinë lietuviø kalba pasiþymi dideliu archajiðkumu (ypaè vardaþodþiø linksniavimo srityje).[3] Fonetiðkai ir morfologiðkai konservatyvi lietuviø kalba þymiai artimesnë baltø prokalbei negu naujoviðkesnë latviø kalba.[4] Lietuviø kalba – archajiðkiausia ið gyvøjø indoeuropieèiø kalbø, iðsaugojusi daugybæ indoeuropieèiø prokalbës ypatybiø.[5]
            Lietuviø kalba skirstoma á dvi pagrindines tarmes: aukðtaièiø ir þemaièiø. Dabartinë bendrinë lietuviø kalba grindþiama vakarø aukðtaièiø kauniðkiø patarme.",
            vec![&Language::Estonian],
        ),
        (
            "In a statement by players' union Futpro, which is representing 33-year-old Hermoso, she is quoted as saying in no case did I seek to raise (lift) the president while they embraced on the podium.
            The Spanish Football Federation (RFEF) said: The RFEF and Mr President will demonstrate each of the lies that are spread either by someone on behalf of the player or, if applicable, by the player hersel. Bсеки човек има право на образование. Oбразованието трябва да бъде безплатно, поне що се отнася до началното и основното образование.",
            vec![&Language::Bulgarian, &Language::English],
        ),
    ];

    for (text, expected_languages) in tests {
        let result = coherence_ratio(text.to_string(), None, None).unwrap();
        for lang in expected_languages {
            assert!(result.iter().any(|cm| cm.language == lang));
        }
    }
}
