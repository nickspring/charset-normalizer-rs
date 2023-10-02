use crate::from_bytes;

#[test]
fn test_undefined_unicode_ranges() {
    let tests = [b"\xef\xbb\xbf\xf0\x9f\xa9\xb3".as_slice()];

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
            "utf-8",
            "UTF-8 payload wrongly detected! Input: {:?}",
            &input
        );
        assert_eq!(
            best_guess.unwrap().unicode_ranges().len(),
            0,
            "This property in that edge case, should return a empty list. Input: {:?}",
            &input
        );
    }
}
