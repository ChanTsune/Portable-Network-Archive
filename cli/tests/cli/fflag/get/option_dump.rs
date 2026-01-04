use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: An archive with entries that have file flags set.
/// Action: Run `pna experimental fflag get --dump` to output restorable format.
/// Expectation: Output is in the format `# file: path\nflags=flag1,flag2`.
#[test]
fn fflag_get_dump_format() {
    setup();
    fs::create_dir_all("fflag_get_dump").unwrap();

    fs::write("fflag_get_dump/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_dump/test.pna",
            "--overwrite",
            "fflag_get_dump/testfile.txt",
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
            "fflag_get_dump/test.pna",
            "uchg,nodump,hidden",
            "fflag_get_dump/testfile.txt",
        ])
        .assert()
        .success();

    // Get with dump format
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_dump/test.pna",
            "--dump",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "# file: fflag_get_dump/testfile.txt",
        ))
        .stdout(predicates::str::contains("flags="));
}

/// Precondition: An archive with entries that have no flags.
/// Action: Run `pna experimental fflag get --dump`.
/// Expectation: Output shows file header with empty flags.
#[test]
fn fflag_get_dump_empty_flags() {
    setup();
    fs::create_dir_all("fflag_get_dump_empty").unwrap();

    fs::write("fflag_get_dump_empty/testfile.txt", "test content").unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_dump_empty/test.pna",
            "--overwrite",
            "fflag_get_dump_empty/testfile.txt",
        ])
        .assert()
        .success();

    // Get with dump format - should show file with empty flags
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_dump_empty/test.pna",
            "--dump",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("# file:"))
        .stdout(predicates::str::contains("flags="));
}
