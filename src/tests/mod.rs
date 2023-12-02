#![cfg(test)]
use crate::entity::Language;
use once_cell::sync::Lazy;
mod cd;
mod detection_base;
mod detection_edge_case;
mod detection_full;
mod detection_large_payload;
mod entity;
mod md;
mod utils;

pub static FILES_SAMPLES: Lazy<Vec<(&'static str, Vec<&'static str>, &'static Language)>> =
    Lazy::new(|| {
        vec![
            (
                "sample-turkish.txt",
                vec!["windows-1254"],
                &Language::Turkish,
            ),
            ("sample-chinese.txt", vec!["big5"], &Language::Chinese),
            (
                "sample-french-1.txt",
                vec!["iso-8859-1", "windows-1252"],
                &Language::French,
            ),
            (
                "sample-arabic-1.txt",
                vec!["windows-1256"],
                &Language::Arabic,
            ),
            ("sample-arabic.txt", vec!["utf-8"], &Language::Arabic),
            (
                "sample-greek.txt",
                vec!["windows-1253", "iso-8859-7"],
                &Language::Greek,
            ),
            ("sample-french.txt", vec!["utf-8"], &Language::French),
            ("sample-russian-3.txt", vec!["utf-8"], &Language::Russian),
            (
                "sample-greek-2.txt",
                vec!["windows-1253", "iso-8859-7"],
                &Language::Greek,
            ),
            (
                "sample-hebrew-2.txt",
                vec!["windows-1255", "iso-8859-8"],
                &Language::Hebrew,
            ),
            (
                "sample-hebrew-3.txt",
                vec!["windows-1255", "iso-8859-8"],
                &Language::Hebrew,
            ),
            ("sample-bulgarian.txt", vec!["utf-8"], &Language::Bulgarian),
            ("sample-english.bom.txt", vec!["utf-8"], &Language::English),
            ("sample-spanish.txt", vec!["utf-8"], &Language::Spanish),
            ("sample-korean.txt", vec!["euc-kr"], &Language::Korean),
            ("sample-russian-2.txt", vec!["utf-8"], &Language::Russian),
            (
                "sample-russian.txt",
                vec!["x-mac-cyrillic"],
                &Language::Russian,
            ),
            ("sample-polish.txt", vec!["utf-8"], &Language::Polish),
        ]
    });
