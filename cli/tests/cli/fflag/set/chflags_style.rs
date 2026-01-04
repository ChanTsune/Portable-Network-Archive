use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive with entries.
/// Action: Use chflags-style flag syntax like "uchg" and "nouchg".
/// Expectation: Flags are correctly set and cleared based on syntax.
#[test]
fn fflag_set_chflags_style_set() {
    setup();
    fs::create_dir_all("fflag_chflags_set").unwrap();

    fs::write("fflag_chflags_set/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_chflags_set/test.pna",
            "--overwrite",
            "fflag_chflags_set/testfile.txt",
        ])
        .assert()
        .success();

    // Set flag using chflags-style syntax
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_chflags_set/test.pna",
            "uchg",
            "fflag_chflags_set/testfile.txt",
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
            "fflag_chflags_set/test.pna",
            "fflag_chflags_set/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"));
}

/// Precondition: An archive with entry having flags set.
/// Action: Use "no" prefix to clear a flag (e.g., "nouchg").
/// Expectation: Flag is removed from the entry.
#[test]
fn fflag_set_chflags_style_clear() {
    setup();
    fs::create_dir_all("fflag_chflags_clear").unwrap();

    fs::write("fflag_chflags_clear/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_chflags_clear/test.pna",
            "--overwrite",
            "fflag_chflags_clear/testfile.txt",
        ])
        .assert()
        .success();

    // Set multiple flags
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_chflags_clear/test.pna",
            "uchg,nodump",
            "fflag_chflags_clear/testfile.txt",
        ])
        .assert()
        .success();

    // Clear uchg using "nouchg"
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_chflags_clear/test.pna",
            "nouchg",
            "fflag_chflags_clear/testfile.txt",
        ])
        .assert()
        .success();

    // Verify uchg is cleared but nodump remains
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_chflags_clear/test.pna",
            "fflag_chflags_clear/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg").not())
        .stdout(predicates::str::contains("nodump"));
}

/// Precondition: nodump flag uses special syntax "dump" to clear.
/// Action: Use "dump" to clear nodump flag.
/// Expectation: nodump flag is removed.
#[test]
fn fflag_set_dump_clears_nodump() {
    setup();
    fs::create_dir_all("fflag_dump_clear").unwrap();

    fs::write("fflag_dump_clear/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_dump_clear/test.pna",
            "--overwrite",
            "fflag_dump_clear/testfile.txt",
        ])
        .assert()
        .success();

    // Set nodump flag
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_dump_clear/test.pna",
            "nodump",
            "fflag_dump_clear/testfile.txt",
        ])
        .assert()
        .success();

    // Clear using "dump"
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_dump_clear/test.pna",
            "dump",
            "fflag_dump_clear/testfile.txt",
        ])
        .assert()
        .success();

    // Verify nodump is cleared
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_dump_clear/test.pna",
            "fflag_dump_clear/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("nodump").not());
}
