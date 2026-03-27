use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive entry with multiple flags.
/// Action: Clear all flags by setting empty or using no-prefixes.
/// Expectation: All specified flags are cleared.
#[test]
fn fflag_clear_single_flag() {
    setup();
    fs::create_dir_all("fflag_clear_single").unwrap();

    fs::write("fflag_clear_single/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_clear_single/test.pna",
            "--overwrite",
            "fflag_clear_single/testfile.txt",
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
            "fflag_clear_single/test.pna",
            "uchg,hidden",
            "fflag_clear_single/testfile.txt",
        ])
        .assert()
        .success();

    // Clear single flag
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_clear_single/test.pna",
            "nouchg",
            "fflag_clear_single/testfile.txt",
        ])
        .assert()
        .success();

    // Verify only hidden remains
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_clear_single/test.pna",
            "fflag_clear_single/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg").not())
        .stdout(predicates::str::contains("hidden"));
}

/// Precondition: An archive entry with multiple flags.
/// Action: Clear multiple flags at once.
/// Expectation: All specified flags are cleared.
#[test]
fn fflag_clear_multiple_flags() {
    setup();
    fs::create_dir_all("fflag_clear_multi").unwrap();

    fs::write("fflag_clear_multi/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_clear_multi/test.pna",
            "--overwrite",
            "fflag_clear_multi/testfile.txt",
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
            "fflag_clear_multi/test.pna",
            "uchg,nodump,hidden",
            "fflag_clear_multi/testfile.txt",
        ])
        .assert()
        .success();

    // Clear multiple flags
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_clear_multi/test.pna",
            "nouchg,nohidden",
            "fflag_clear_multi/testfile.txt",
        ])
        .assert()
        .success();

    // Verify only nodump remains
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_clear_multi/test.pna",
            "fflag_clear_multi/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg").not())
        .stdout(predicates::str::contains("hidden").not())
        .stdout(predicates::str::contains("nodump"));
}

/// Precondition: An archive entry.
/// Action: Try to clear a flag that's not set.
/// Expectation: Command succeeds (no-op).
#[test]
fn fflag_clear_nonexistent_flag() {
    setup();
    fs::create_dir_all("fflag_clear_nonexistent").unwrap();

    fs::write("fflag_clear_nonexistent/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_clear_nonexistent/test.pna",
            "--overwrite",
            "fflag_clear_nonexistent/testfile.txt",
        ])
        .assert()
        .success();

    // Try to clear a flag that doesn't exist
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_clear_nonexistent/test.pna",
            "nouchg",
            "fflag_clear_nonexistent/testfile.txt",
        ])
        .assert()
        .success();
}
