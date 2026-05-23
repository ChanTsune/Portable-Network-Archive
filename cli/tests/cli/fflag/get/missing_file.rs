use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: Archive file does not exist.
/// Action: Run `pna experimental fflag get` on non-existent archive.
/// Expectation: Command fails with appropriate error.
#[test]
fn fflag_get_missing_archive() {
    setup();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "nonexistent.pna",
            "*",
        ])
        .assert()
        .failure();
}

/// Precondition: Archive exists but entry does not.
/// Action: Run `pna experimental fflag get` on non-existent entry.
/// Expectation: Command fails with appropriate error.
#[test]
fn fflag_get_missing_entry() {
    setup();
    std::fs::create_dir_all("fflag_get_missing_entry").unwrap();
    std::fs::write("fflag_get_missing_entry/file.txt", "content").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_get_missing_entry/test.pna",
            "--overwrite",
            "fflag_get_missing_entry/file.txt",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_get_missing_entry/test.pna",
            "nonexistent.txt",
        ])
        .assert()
        .failure();
}
