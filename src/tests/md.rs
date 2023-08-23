use crate::md::*;
use crate::utils::{decode, get_large_test_datasets};
use encoding::DecoderTrap;
use ordered_float::OrderedFloat;
use std::fs::File;
use std::io::Read;

#[test]
fn test_mess_ratio() {
    let tests =     [
        // content, min_expected_ratio, max_expected_ratio
        ("典肇乎庚辰年十二月廿一，及己丑年二月十九，收各方語言二百五十，合逾七百萬目；二十大卷佔八成，單英文卷亦過二百萬。悉文乃天下有志共筆而成；有意助之，幾網路、隨纂作，大典茁焉。", 0.0, 0.0),
        ("العقلية , التنويم المغناطيسي و / أو الاقتراح", 0.0, 0.0),
        ("RadoZ تـــعــــديــل الـــتــــوقــيــــت مـــن قــبــل", 0.0, 0.0),
        ("Cehennemin Sava■þ²s²'da kim?", 0.1, 0.5),
        ("´Á¥½³ø§i --  ±i®Ìºû, ³¯·Ø©v", 0.5, 1.0),
        ("ïstanbul, T■rkiye'nin en kalabal»k, iktisadi ve k■lt■rel aÓ»dan en —nemli", 0.1, 0.501),
        ("<i>Parce que Óa, c'est la vÕritable histoire de la rencontre avec votre Tante Robin.</i>", 0.01, 0.5),
        ("ØĢØŠØģØ§ØĶŲ ŲŲ ØĢŲ Ø§ŲŲØ§Øģ ŲŲŲ ŲØ§ ØģŲŲŲØŠØģØ§ØĶŲŲŲØ ØŊØđŲØ§ ŲØģŲØđ ØđŲ (ŲØąŲØŊŲ) ŲØ§ŲØŪØ§ØŠŲ", 0.8, 2.0),
        ("ÇáÚŞáíÉ , ÇáÊäæíã ÇáãÛäÇØíÓí æ / Ãæ ÇáÇŞÊÑÇÍ", 0.8, 2.5),
        ("hishamkoc@yahoo.com ุชุฑุฌูููุฉ ููุดูููุงู ุงููููููููุงูRadoZ ุชูููุนููููุฏูููู ุงููููุชูููููููููููููุช ููููู ูููุจููู", 0.5, 2.0),
    ];
    for test in &tests {
        let mr = mess_ratio(test.0.to_string(), Some(OrderedFloat(1.0)));
        assert!(
            mr >= test.1 && mr <= test.2,
            "The mess detection ratio {} calculated for given content is not well adjusted: {}",
            mr,
            test.0
        );
    }
}

#[test]
fn test_datasets_mess_ratio() {
    for (path, encoding) in &get_large_test_datasets().unwrap() {
        let file = File::open(path);
        if file.is_err() {
            return;
        }
        let mut buffer = Vec::new();
        if file.unwrap().read_to_end(&mut buffer).is_err() {
            return;
        }
        if let Ok(decoded_sequence) = decode(
            &buffer,
            encoding.first().unwrap(),
            DecoderTrap::Ignore,
            false,
            false,
        ) {
            let mr = mess_ratio(decoded_sequence, Some(OrderedFloat(1.0)));
            assert!(mr < 0.2);
        }
    }
}
