use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive with multiple entries.
/// Action: Set flags on multiple entries using glob pattern.
/// Expectation: All matching entries get the flags.
#[test]
fn fflag_set_multiple_entries_glob() {
    setup();
    fs::create_dir_all("fflag_set_glob").unwrap();

    fs::write("fflag_set_glob/file1.txt", "content 1").unwrap();
    fs::write("fflag_set_glob/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_glob/test.pna",
            "--overwrite",
            "fflag_set_glob/file1.txt",
            "fflag_set_glob/file2.txt",
        ])
        .assert()
        .success();

    // Set flags on all entries using glob
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_glob/test.pna",
            "nodump",
            "*",
        ])
        .assert()
        .success();

    // Verify both entries have the flag
    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_glob/test.pna",
            "--long",
            "*",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("file1.txt"));
    assert!(output_str.contains("file2.txt"));
    assert!(output_str.matches("nodump").count() >= 2);
}

/// Precondition: An archive with multiple entries.
/// Action: Set different flags on different entries.
/// Expectation: Each entry has its own flags.
#[test]
fn fflag_set_different_flags_per_entry() {
    setup();
    fs::create_dir_all("fflag_set_different").unwrap();

    fs::write("fflag_set_different/file1.txt", "content 1").unwrap();
    fs::write("fflag_set_different/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_different/test.pna",
            "--overwrite",
            "fflag_set_different/file1.txt",
            "fflag_set_different/file2.txt",
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
            "fflag_set_different/test.pna",
            "uchg",
            "fflag_set_different/file1.txt",
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
            "fflag_set_different/test.pna",
            "hidden",
            "fflag_set_different/file2.txt",
        ])
        .assert()
        .success();

    // Verify file1 has uchg
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_different/test.pna",
            "fflag_set_different/file1.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("hidden").not());

    // Verify file2 has hidden
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_different/test.pna",
            "fflag_set_different/file2.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("hidden"))
        .stdout(predicates::str::contains("uchg").not());
}

/// Precondition: An archive with multiple entries.
/// Action: Set flags on specific entries by name.
/// Expectation: Only named entries are modified.
#[test]
fn fflag_set_specific_entries_only() {
    setup();
    fs::create_dir_all("fflag_set_specific").unwrap();

    fs::write("fflag_set_specific/file1.txt", "content 1").unwrap();
    fs::write("fflag_set_specific/file2.txt", "content 2").unwrap();
    fs::write("fflag_set_specific/file3.txt", "content 3").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_specific/test.pna",
            "--overwrite",
            "fflag_set_specific/file1.txt",
            "fflag_set_specific/file2.txt",
            "fflag_set_specific/file3.txt",
        ])
        .assert()
        .success();

    // Set flag on file1 and file3 only
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_specific/test.pna",
            "nodump",
            "fflag_set_specific/file1.txt",
            "fflag_set_specific/file3.txt",
        ])
        .assert()
        .success();

    // Get all flags
    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_specific/test.pna",
            "--long",
            "*",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.contains("file1.txt"));
    assert!(!output_str.contains("file2.txt")); // file2 should not appear (no flags)
    assert!(output_str.contains("file3.txt"));
}
