use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: An archive with entries.
/// Action: Run `pna experimental fflag set` to set flags on an entry.
/// Expectation: Flags are stored in the archive.
#[test]
fn fflag_set_basic() {
    setup();
    fs::create_dir_all("fflag_set_basic").unwrap();

    fs::write("fflag_set_basic/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_basic/test.pna",
            "--overwrite",
            "fflag_set_basic/testfile.txt",
        ])
        .assert()
        .success();

    // Set a single flag
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_basic/test.pna",
            "uchg",
            "fflag_set_basic/testfile.txt",
        ])
        .assert()
        .success();

    // Verify flag is set
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_basic/test.pna",
            "fflag_set_basic/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"));
}

/// Precondition: An archive with entries.
/// Action: Set multiple flags with comma-separated values.
/// Expectation: All specified flags are stored.
#[test]
fn fflag_set_multiple_flags() {
    setup();
    fs::create_dir_all("fflag_set_multi").unwrap();

    fs::write("fflag_set_multi/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_multi/test.pna",
            "--overwrite",
            "fflag_set_multi/testfile.txt",
        ])
        .assert()
        .success();

    // Set multiple flags at once
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_multi/test.pna",
            "uchg,nodump,hidden",
            "fflag_set_multi/testfile.txt",
        ])
        .assert()
        .success();

    // Verify all flags are set
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_multi/test.pna",
            "fflag_set_multi/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("nodump"))
        .stdout(predicates::str::contains("hidden"));
}

/// Precondition: An archive entry already has flags.
/// Action: Set additional flags.
/// Expectation: New flags are added, existing flags remain.
#[test]
fn fflag_set_additive() {
    setup();
    fs::create_dir_all("fflag_set_additive").unwrap();

    fs::write("fflag_set_additive/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_additive/test.pna",
            "--overwrite",
            "fflag_set_additive/testfile.txt",
        ])
        .assert()
        .success();

    // Set first flag
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_additive/test.pna",
            "uchg",
            "fflag_set_additive/testfile.txt",
        ])
        .assert()
        .success();

    // Set another flag
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_additive/test.pna",
            "nodump",
            "fflag_set_additive/testfile.txt",
        ])
        .assert()
        .success();

    // Verify both flags are present
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_additive/test.pna",
            "fflag_set_additive/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("nodump"));
}
