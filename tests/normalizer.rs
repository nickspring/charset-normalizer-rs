use assert_cmd::Command;
use predicates::prelude::*;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

fn get_sample_path(sample_name: &str) -> OsString {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(format!("src/tests/data/samples/{}", sample_name));
    path.as_os_str().to_os_string()
}

#[test]
fn test_cli_single_file() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[get_sample_path("sample-arabic-1.txt")])
        .assert()
        .success()
        .code(predicate::eq(0))
        .stdout(predicate::str::contains("language\": \"Arabic\""));
}

#[test]
fn test_cli_version_output_success() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(["--version"])
        .assert()
        .success()
        .code(predicate::eq(0))
        .stdout(predicate::str::contains(
            "The Real First Universal Charset Detector",
        ));
}

#[test]
fn test_cli_single_file_normalize() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        get_sample_path("sample-arabic-1.txt"),
        OsString::from("--normalize"),
    ])
    .assert()
    .success()
    .code(predicate::eq(0))
    .stdout(predicate::str::contains("language\": \"Arabic\""));

    let normalized_path = &get_sample_path("sample-arabic-1.windows-1256.txt");
    assert!(fs::metadata(normalized_path).is_ok());
    fs::remove_file(normalized_path).expect("Normalized file is not exists");
}

#[test]
fn test_cli_single_verbose_file() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        get_sample_path("sample-arabic-1.txt"),
        OsString::from("--verbose"),
    ])
    .assert()
    .success()
    .code(predicate::eq(0))
    .stdout(predicate::str::contains("language\": \"Arabic\""));
}

#[test]
fn test_cli_multiple_files() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        get_sample_path("sample-arabic-1.txt"),
        get_sample_path("sample-french.txt"),
        get_sample_path("sample-chinese.txt"),
    ])
    .assert()
    .success()
    .code(predicate::eq(0));
}

#[test]
fn test_cli_multiple_files_with_alternative() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        OsString::from("-a"),
        get_sample_path("sample-arabic-1.txt"),
        get_sample_path("sample-french.txt"),
        get_sample_path("sample-chinese.txt"),
    ])
    .assert()
    .success()
    .code(predicate::eq(0));
}

#[test]
fn test_cli_multiple_files_with_minimal_output() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        OsString::from("-m"),
        get_sample_path("sample-arabic-1.txt"),
        get_sample_path("sample-french.txt"),
        get_sample_path("sample-chinese.txt"),
    ])
    .assert()
    .success()
    .code(predicate::eq(0));
}

#[test]
fn test_cli_non_existent_file() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[get_sample_path("non-exists-file.txt")])
        .assert()
        .failure()
        .code(predicate::gt(0));
}

#[test]
fn test_cli_replace_without_normalize() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        OsString::from("--replace"),
        get_sample_path("sample-arabic-1.txt"),
    ])
    .assert()
    .failure()
    .code(predicate::gt(0));
}

#[test]
fn test_cli_force_replace_without_replace() {
    let mut cmd = Command::cargo_bin("normalizer").unwrap();
    cmd.args(&[
        OsString::from("--replace"),
        get_sample_path("sample-arabic-1.txt"),
    ])
    .assert()
    .failure()
    .code(predicate::gt(0));
}
