use crate::utils::{create_high_cost_encrypted_archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

const PASSWORD: &str = "testpass";

fn prepare(name: &str) -> (String, String) {
    let dir = format!("extract_kdf_limit_{name}");
    let _ = fs::remove_dir_all(&dir);
    let archive = format!("{dir}/source.pna");
    create_high_cost_encrypted_archive(&archive, PASSWORD, "secret.txt", b"secret content");
    (archive, format!("{dir}/out"))
}

/// Precondition: An encrypted archive uses KDF parallelism above the bounded cap.
/// Action: Extract it without a KDF cost limit flag.
/// Expectation: Extraction succeeds and the entry contents are intact.
#[test]
fn extract_default_accepts_high_cost_archive() {
    setup();
    let (archive, out_dir) = prepare("default");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "x",
            "-f",
            &archive,
            "--out-dir",
            &out_dir,
            "--password",
            PASSWORD,
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read(format!("{out_dir}/secret.txt")).unwrap(),
        b"secret content"
    );
}

/// Precondition: An encrypted archive uses KDF parallelism above the bounded cap.
/// Action: Extract it with `--limit-kdf`.
/// Expectation: Extraction fails because the KDF cost exceeds the cap.
#[test]
fn extract_with_limit_kdf_rejects_high_cost_archive() {
    setup();
    let (archive, out_dir) = prepare("bounded");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "x",
            "-f",
            &archive,
            "--out-dir",
            &out_dir,
            "--password",
            PASSWORD,
            "--limit-kdf",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds maximum"));
}

/// Precondition: An encrypted archive uses KDF parallelism above the bounded cap.
/// Action: Extract it with `--no-limit-kdf`.
/// Expectation: Extraction succeeds and the entry contents are intact.
#[test]
fn extract_with_no_limit_kdf_accepts_high_cost_archive() {
    setup();
    let (archive, out_dir) = prepare("unbounded");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "x",
            "-f",
            &archive,
            "--out-dir",
            &out_dir,
            "--password",
            PASSWORD,
            "--no-limit-kdf",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read(format!("{out_dir}/secret.txt")).unwrap(),
        b"secret content"
    );
}

/// Precondition: A command accepts both KDF limit flags.
/// Action: Pass both `--limit-kdf` and `--no-limit-kdf` to extract.
/// Expectation: The command is rejected as a conflicting option pair.
#[test]
fn extract_rejects_both_kdf_flags() {
    setup();
    let (archive, out_dir) = prepare("conflict");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "x",
            "-f",
            &archive,
            "--out-dir",
            &out_dir,
            "--password",
            PASSWORD,
            "--limit-kdf",
            "--no-limit-kdf",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}
