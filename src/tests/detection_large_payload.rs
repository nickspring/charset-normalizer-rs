use crate::consts::TOO_BIG_SEQUENCE;
use crate::from_bytes;

#[test]
fn test_large_payload_utf8_sig_basic_entry() {
    let mut payload = b"\xef\xbb\xbf".as_slice().to_vec();
    payload.extend(b"0".repeat(TOO_BIG_SEQUENCE + 1).as_slice().to_vec());

    let result = from_bytes(&payload, None);
    let best_guess = result.get_best();
    assert!(
        best_guess.is_some(),
        "Large U8 payload case detection completely failed"
    );
    assert_eq!(
        best_guess.unwrap().encoding(),
        "utf-8",
        "Large U8 payload case detection wrongly detected!"
    );
    assert!(best_guess.unwrap().bom(), "SIG/BOM property should be True");
    assert_eq!(
        best_guess.unwrap().raw().len(),
        payload.len(),
        "Large payload should remain untouched when accessed through .raw"
    );
}

#[test]
fn test_large_payload_ascii_sig_basic_entry() {
    let payload = b"0".repeat(TOO_BIG_SEQUENCE + 1).as_slice().to_vec();

    let result = from_bytes(&payload, None);
    let best_guess = result.get_best();
    assert!(
        best_guess.is_some(),
        "Large ASCII payload case detection completely failed"
    );
    assert_eq!(
        best_guess.unwrap().encoding(),
        "ascii",
        "Large ASCII payload case detection wrongly detected!"
    );
    assert!(
        !best_guess.unwrap().bom(),
        "SIG/BOM property should be False"
    );
    assert_eq!(
        best_guess.unwrap().raw().len(),
        payload.len(),
        "Large payload should remain untouched when accessed through .raw"
    );
}

#[test]
fn test_misleading_large_sequence() {
    let mut payload = b"hello simple ascii "
        .repeat(TOO_BIG_SEQUENCE)
        .as_slice()
        .to_vec();
    payload.extend("我没有埋怨，磋砣的只是一些时间。 磋砣的只是一些时间。".as_bytes());

    let result = from_bytes(&payload, None);
    assert!(!result.is_empty(), "No results");
    let best_guess = result.get_best();
    assert!(best_guess.is_some(), "Best guess is exists");
    assert_eq!(
        best_guess.unwrap().encoding(),
        "utf-8",
        "Best guess is not utf-8"
    );
    assert!(
        best_guess.unwrap().decoded_payload().is_some(),
        "Decoded content is empty"
    );
}
