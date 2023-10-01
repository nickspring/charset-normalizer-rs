use crate::entity::NormalizerSettings;
use crate::from_bytes;
use crate::utils::encode;
use encoding::EncoderTrap;

#[test]
fn test_empty() {
    let bytes: Vec<u8> = b"".to_vec();
    let result = from_bytes(&bytes, None);
    let best_guess = result.get_best();

    assert!(
        best_guess.is_some(),
        "Empty bytes payload SHOULD NOT return None"
    );
    assert_eq!(
        best_guess.unwrap().encoding(),
        "utf-8",
        "Empty bytes payload SHOULD be guessed as UTF-8 (arbitrary)"
    );
    assert!(best_guess.unwrap().unicode_ranges().is_empty());
}

#[test]
fn test_empty_but_with_bom_or_sig() {
    let tests = [
        (vec![0xff, 0xfe], "utf-16le"),
        (vec![0x84, 0x31, 0x95, 0x33], "gb18030"),
        (vec![0xef, 0xbb, 0xbf], "utf-8"),
        (vec![0xfe, 0xff], "utf-16be"),
    ];

    for (input, expected_encoding) in tests {
        let result = from_bytes(&input, None);
        let best_guess = result.get_best();
        assert!(
            best_guess.is_some(),
            "Empty detection but with SIG/BOM has failed! Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().encoding(),
            expected_encoding,
            "Empty detection but with SIG/BOM is wrongly detected! Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().raw(),
            &input,
            "The RAW property should contain the original payload given for detection. Input: {:?}",
            &input
        );
        assert!(
            best_guess.unwrap().bom(),
            "The BOM/SIG property should return True. Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().decoded_payload(),
            Some(""),
            "The cast to str SHOULD be empty. Input: {:?}",
            &input
        );
    }
}

#[test]
fn test_content_with_bom_or_sig() {
    let tests = [
        (
            encode(
                "\u{FEFF}我没有埋怨，磋砣的只是一些时间。",
                "gb18030",
                EncoderTrap::Ignore,
            )
            .unwrap(),
            "gb18030",
        ),
        (
            encode(
                "\u{FEFF}我没有埋怨，磋砣的只是一些时间。",
                "utf-16le",
                EncoderTrap::Ignore,
            )
            .unwrap(),
            "utf-16le",
        ),
        (
            encode(
                "\u{FEFF}我没有埋怨，磋砣的只是一些时间。",
                "utf-8",
                EncoderTrap::Ignore,
            )
            .unwrap(),
            "utf-8",
        ),
    ];

    for (input, expected_encoding) in tests {
        let result = from_bytes(&input, None);
        let best_guess = result.get_best();
        assert!(
            best_guess.is_some(),
            "Detection but with SIG/BOM has failed! Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().encoding(),
            expected_encoding,
            "Detection but with SIG/BOM is wrongly detected! Input: {:?}",
            &input
        );
        assert!(
            best_guess.unwrap().bom(),
            "The BOM/SIG property should return True. Input: {:?}",
            &input
        );
    }
}

#[test]
fn test_obviously_ascii_content() {
    let tests = [
        b"AbAdZ pOoooOlDl mmlDoDkA lldDkeEkddA mpAlkDF".as_slice(),
        b"g4UsPJdfzNkGW2jwmKDGDilKGKYtpF2X.mx3MaTWL1tL7CNn5U7DeCcodKX7S3lwwJPKNjBT8etY".as_slice(),
        b"{\"token\": \"g4UsPJdfzNkGW2jwmKDGDilKGKYtpF2X.mx3MaTWL1tL7CNn5U7DeCcodKX7S3lwwJPKNjBT8etY\"}".as_slice(),
        b"81f4ab054b39cb0e12701e734077d84264308f5fc79494fc5f159fa2ebc07b73c8cc0e98e009664a20986706f90146e8eefcb929ce1f74a8eab21369fdc70198".as_slice(),
        b"{}".as_slice(),
    ];

    for input in tests {
        let result = from_bytes(input, None);
        let best_guess = result.get_best();
        assert!(
            best_guess.is_some(),
            "Dead-simple ASCII detection has failed! Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().encoding(),
            "ascii",
            "Dead-simple ASCII detection is wrongly detected! Input: {:?}",
            &input
        );
    }
}

#[test]
fn test_obviously_utf8_content() {
    let tests = [
        "\u{020d}",
        "héllo world!\n",
        "我没有埋怨，磋砣的只是一些时间。",
        "Bсеки човек има право на образование. Oбразованието трябва да бъде безплатно, поне що се отнася до началното и основното образование.",
        "Bсеки човек има право на образование.",
        "(° ͜ʖ °), creepy face, smiley 😀",
        "[\"Financiën\", \"La France\"]",
        "Qu'est ce que une étoile?",
        "<?xml ?><c>Financiën</c>",
        "😀",
    ];

    for input in tests {
        let result = from_bytes(input.as_bytes(), None);
        let best_guess = result.get_best();
        assert!(
            best_guess.is_some(),
            "Dead-simple UTF-8 detection has failed! Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().encoding(),
            "utf-8",
            "Dead-simple UTF-8 detection is wrongly detected! Input: {:?}",
            &input
        );
    }
}

#[test]
fn test_unicode_ranges_property() {
    let text = "😀 Hello World! How affairs are going? 😀";
    let result = from_bytes(text.as_bytes(), None);
    let best_guess = result.get_best();
    let ur = best_guess.unwrap().unicode_ranges();
    assert!(ur.contains(&"Basic Latin".to_string()));
    assert!(ur.contains(&"Emoticons range(Emoji)".to_string()));
}

#[test]
fn test_mb_cutting_chk() {
    let payload = b"\xbf\xaa\xbb\xe7\xc0\xfb    \xbf\xb9\xbc\xf6    \xbf\xac\xb1\xb8\xc0\xda\xb5\xe9\xc0\xba  \xba\xb9\xc0\xbd\xbc\xad\xb3\xaa ".repeat(128);
    let mut settings = NormalizerSettings::default();
    settings.include_encodings.push(String::from("euc-kr"));
    let result = from_bytes(payload.as_slice(), Some(settings));
    let best_guess = result.get_best().unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(best_guess.encoding(), "euc-kr");
}
