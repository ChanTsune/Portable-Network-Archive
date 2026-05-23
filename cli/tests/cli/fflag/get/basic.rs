use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive with entries that have file flags set.
/// Action: Run `pna experimental fflag get` to list flags.
/// Expectation: Flags are displayed for entries that have them.
#[test]
fn fflag_get_basic() {
    setup();
    fs::create_dir_all("fflag_get_basic").unwrap();

    fs::write("fflag_get_basic/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_basic/test.pna",
            "--overwrite",
            "fflag_get_basic/testfile.txt",
        ])
        .assert()
        .success();

    // Set flags on the entry
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_basic/test.pna",
            "uchg,nodump",
            "fflag_get_basic/testfile.txt",
        ])
        .assert()
        .success();

    // Get flags
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_basic/test.pna",
            "fflag_get_basic/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("nodump"))
        .stdout(predicates::str::contains("uchg"));
}

/// Precondition: An archive with entries that have no file flags.
/// Action: Run `pna experimental fflag get` to list flags.
/// Expectation: No output for entries without flags.
#[test]
fn fflag_get_no_flags() {
    setup();
    fs::create_dir_all("fflag_get_no_flags").unwrap();

    fs::write("fflag_get_no_flags/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_no_flags/test.pna",
            "--overwrite",
            "fflag_get_no_flags/testfile.txt",
        ])
        .assert()
        .success();

    // Get flags - should have no output since no flags are set
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_no_flags/test.pna",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

/// Precondition: An archive with multiple entries, some with flags.
/// Action: Run `pna experimental fflag get *` to list all flags.
/// Expectation: Only entries with flags are shown.
#[test]
fn fflag_get_wildcard() {
    setup();
    fs::create_dir_all("fflag_get_wildcard").unwrap();

    fs::write("fflag_get_wildcard/file1.txt", "content 1").unwrap();
    fs::write("fflag_get_wildcard/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_wildcard/test.pna",
            "--overwrite",
            "fflag_get_wildcard/file1.txt",
            "fflag_get_wildcard/file2.txt",
        ])
        .assert()
        .success();

    // Set flags only on file1
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_wildcard/test.pna",
            "hidden",
            "fflag_get_wildcard/file1.txt",
        ])
        .assert()
        .success();

    // Get all flags with wildcard
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_wildcard/test.pna",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("file1.txt"))
        .stdout(predicates::str::contains("hidden"))
        .stdout(predicates::str::contains("file2.txt").not());
}
