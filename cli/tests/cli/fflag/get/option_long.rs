use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: An archive with entries that have file flags set.
/// Action: Run `pna experimental fflag get --long` to get verbose output.
/// Expectation: Output includes detailed information about flags.
#[test]
fn fflag_get_long_format() {
    setup();
    fs::create_dir_all("fflag_get_long").unwrap();

    fs::write("fflag_get_long/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_long/test.pna",
            "--overwrite",
            "fflag_get_long/testfile.txt",
        ])
        .assert()
        .success();

    // Set flags
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_long/test.pna",
            "uchg,nodump",
            "fflag_get_long/testfile.txt",
        ])
        .assert()
        .success();

    // Get with long format
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_long/test.pna",
            "--long",
            "fflag_get_long/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("fflag_get_long/testfile.txt"))
        .stdout(predicates::str::contains("nodump"))
        .stdout(predicates::str::contains("uchg"));
}

/// Precondition: An archive with multiple entries having different flags.
/// Action: Run `pna experimental fflag get --long *` to list all flags.
/// Expectation: All entries with flags are displayed with paths.
#[test]
fn fflag_get_long_multiple_entries() {
    setup();
    fs::create_dir_all("fflag_get_long_multi").unwrap();

    fs::write("fflag_get_long_multi/file1.txt", "content 1").unwrap();
    fs::write("fflag_get_long_multi/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_long_multi/test.pna",
            "--overwrite",
            "fflag_get_long_multi/file1.txt",
            "fflag_get_long_multi/file2.txt",
        ])
        .assert()
        .success();

    // Set different flags on different files
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_long_multi/test.pna",
            "uchg",
            "fflag_get_long_multi/file1.txt",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_get_long_multi/test.pna",
            "hidden",
            "fflag_get_long_multi/file2.txt",
        ])
        .assert()
        .success();

    // Get with long format for all
    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_long_multi/test.pna",
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
    assert!(output_str.contains("uchg"));
    assert!(output_str.contains("file2.txt"));
    assert!(output_str.contains("hidden"));
}
