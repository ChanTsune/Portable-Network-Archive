use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: An encrypted archive matches the filesystem exactly.
/// Action: Run `pna experimental diff` with the correct password.
/// Expectation: Exits successfully and produces no stdout.
#[test]
fn diff_encrypted_without_differences() {
    setup();
    let dir = "diff_encrypted_without_differences_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            &file_path,
            "--password=secret",
            "--aes",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--password=secret",
        ])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: An encrypted archive contains a file whose content is later changed on disk to a different value of the same size.
/// Action: Run `pna experimental diff` with the correct password supplied via `--password-file`.
/// Expectation: Exits with status 1 and reports the difference on stdout, with nothing written to stderr.
#[test]
fn diff_encrypted_with_content_difference() {
    setup();
    let dir = "diff_encrypted_with_content_difference_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            &file_path,
            "--password=secret",
            "--aes",
        ])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();

    let password_file = format!("{dir}/password.txt");
    fs::write(&password_file, "secret").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--password-file",
            &password_file,
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Contents differ"))
        .stderr("");
}

/// Precondition: An encrypted archive matches the filesystem exactly.
/// Action: Run `pna experimental diff` without providing a password.
/// Expectation: Exits with status 2 and writes an error to stderr.
#[test]
fn diff_encrypted_without_password() {
    setup();
    let dir = "diff_encrypted_without_password_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            &file_path,
            "--password=secret",
            "--aes",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty().not());
}
