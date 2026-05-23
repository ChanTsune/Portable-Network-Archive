use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive with entries that have multiple flags set.
/// Action: Run `pna experimental fflag get --name <flag>` to filter by flag name.
/// Expectation: Only entries with that specific flag are shown.
#[test]
fn fflag_get_filter_by_name() {
    setup();
    fs::create_dir_all("fflag_get_filter_name").unwrap();

    fs::write("fflag_get_filter_name/file1.txt", "content 1").unwrap();
    fs::write("fflag_get_filter_name/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_filter_name/test.pna",
            "--overwrite",
            "fflag_get_filter_name/file1.txt",
            "fflag_get_filter_name/file2.txt",
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
            "fflag_get_filter_name/test.pna",
            "uchg",
            "fflag_get_filter_name/file1.txt",
        ])
        .assert()
        .success();

    // Set nodump on file2
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_filter_name/test.pna",
            "nodump",
            "fflag_get_filter_name/file2.txt",
        ])
        .assert()
        .success();

    // Filter by name "uchg" - should only show file1
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_filter_name/test.pna",
            "--name",
            "uchg",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("file1.txt"))
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("file2.txt").not());
}

/// Precondition: An archive with entries that have flags.
/// Action: Run `pna experimental fflag get --name <flag>` with no matching entries.
/// Expectation: Empty output.
#[test]
fn fflag_get_filter_by_name_no_match() {
    setup();
    fs::create_dir_all("fflag_get_filter_name_empty").unwrap();

    fs::write("fflag_get_filter_name_empty/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_filter_name_empty/test.pna",
            "--overwrite",
            "fflag_get_filter_name_empty/testfile.txt",
        ])
        .assert()
        .success();

    // Set nodump
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_filter_name_empty/test.pna",
            "nodump",
            "fflag_get_filter_name_empty/testfile.txt",
        ])
        .assert()
        .success();

    // Filter by name "uchg" - no entries have this flag
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_filter_name_empty/test.pna",
            "--name",
            "uchg",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}
