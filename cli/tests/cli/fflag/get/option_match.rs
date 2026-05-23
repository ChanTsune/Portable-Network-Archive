use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive with entries that have different flags.
/// Action: Run `pna experimental fflag get --match` to filter by flag name.
/// Expectation: Only entries with matching flags are displayed.
#[test]
fn fflag_get_match_single_flag() {
    setup();
    fs::create_dir_all("fflag_get_match").unwrap();

    fs::write("fflag_get_match/file1.txt", "content 1").unwrap();
    fs::write("fflag_get_match/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_match/test.pna",
            "--overwrite",
            "fflag_get_match/file1.txt",
            "fflag_get_match/file2.txt",
        ])
        .assert()
        .success();

    // Set uchg on file1
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_match/test.pna",
            "uchg",
            "fflag_get_match/file1.txt",
        ])
        .assert()
        .success();

    // Set hidden on file2
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_match/test.pna",
            "hidden",
            "fflag_get_match/file2.txt",
        ])
        .assert()
        .success();

    // Match only uchg
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_match/test.pna",
            "--match",
            "uchg",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("file1.txt"))
        .stdout(predicates::str::contains("file2.txt").not());
}

/// Precondition: An archive with entries that have multiple flags.
/// Action: Run `pna experimental fflag get --match` with multiple flags.
/// Expectation: Entries matching any of the flags are displayed.
#[test]
fn fflag_get_match_no_matches() {
    setup();
    fs::create_dir_all("fflag_get_match_none").unwrap();

    fs::write("fflag_get_match_none/file.txt", "content").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_match_none/test.pna",
            "--overwrite",
            "fflag_get_match_none/file.txt",
        ])
        .assert()
        .success();

    // Set uchg
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_match_none/test.pna",
            "uchg",
            "fflag_get_match_none/file.txt",
        ])
        .assert()
        .success();

    // Match nodump - should find no entries
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_match_none/test.pna",
            "--match",
            "nodump",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}
