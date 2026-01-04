use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use std::fs;

/// Precondition: An archive with entry having some flags.
/// Action: Set and clear flags in the same command.
/// Expectation: Both set and clear operations are applied.
#[test]
fn fflag_set_and_clear_in_one_command() {
    setup();
    fs::create_dir_all("fflag_mixed_ops").unwrap();

    fs::write("fflag_mixed_ops/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_mixed_ops/test.pna",
            "--overwrite",
            "fflag_mixed_ops/testfile.txt",
        ])
        .assert()
        .success();

    // Set initial flags
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_mixed_ops/test.pna",
            "uchg,hidden",
            "fflag_mixed_ops/testfile.txt",
        ])
        .assert()
        .success();

    // Mixed operation: clear uchg, set nodump
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_mixed_ops/test.pna",
            "nouchg,nodump",
            "fflag_mixed_ops/testfile.txt",
        ])
        .assert()
        .success();

    // Verify: uchg cleared, hidden unchanged, nodump added
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_mixed_ops/test.pna",
            "fflag_mixed_ops/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg").not())
        .stdout(predicates::str::contains("hidden"))
        .stdout(predicates::str::contains("nodump"));
}

/// Precondition: An archive entry.
/// Action: Set a flag, then toggle it using no-prefix in same invocation.
/// Expectation: Final state depends on processing order.
#[test]
fn fflag_set_same_flag_twice() {
    setup();
    fs::create_dir_all("fflag_same_twice").unwrap();

    fs::write("fflag_same_twice/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_same_twice/test.pna",
            "--overwrite",
            "fflag_same_twice/testfile.txt",
        ])
        .assert()
        .success();

    // Set the same flag twice (should be idempotent)
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_same_twice/test.pna",
            "uchg,uchg",
            "fflag_same_twice/testfile.txt",
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
            "fflag_same_twice/test.pna",
            "fflag_same_twice/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"));
}

/// Precondition: An archive entry with mixed flag operations.
/// Action: Set and clear the same flag in one command.
/// Expectation: Set operations take precedence over clear operations
///              (implementation applies clears first, then sets).
#[test]
fn fflag_set_then_clear_same_flag() {
    setup();
    fs::create_dir_all("fflag_set_clear_same").unwrap();

    fs::write("fflag_set_clear_same/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_set_clear_same/test.pna",
            "--overwrite",
            "fflag_set_clear_same/testfile.txt",
        ])
        .assert()
        .success();

    // Set then clear in same command - set takes precedence
    // (implementation applies clears first, then sets)
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_set_clear_same/test.pna",
            "uchg,nouchg",
            "fflag_set_clear_same/testfile.txt",
        ])
        .assert()
        .success();

    // Verify flag is set (set takes precedence over clear)
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_set_clear_same/test.pna",
            "fflag_set_clear_same/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"));
}
