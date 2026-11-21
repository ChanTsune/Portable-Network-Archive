use crate::utils::{create_high_cost_encrypted_archive, setup};
use clap::{Parser, error::ErrorKind};
use portable_network_archive::cli;
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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive,
        "--out-dir",
        &out_dir,
        "--password",
        PASSWORD,
    ])
    .unwrap()
    .execute()
    .unwrap();

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

    let err = cli::Cli::try_parse_from([
        "pna",
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
    .unwrap()
    .execute()
    .unwrap_err();
    assert!(
        format!("{err:?}").contains("exceeds maximum"),
        "error should be a KDF limit rejection: {err:?}"
    );
}

/// Precondition: An encrypted archive uses KDF parallelism above the bounded cap.
/// Action: Extract it with `--no-limit-kdf`.
/// Expectation: Extraction succeeds and the entry contents are intact.
#[test]
fn extract_with_no_limit_kdf_accepts_high_cost_archive() {
    setup();
    let (archive, out_dir) = prepare("unbounded");

    cli::Cli::try_parse_from([
        "pna",
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
    .unwrap()
    .execute()
    .unwrap();

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

    let err = cli::Cli::try_parse_from([
        "pna",
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
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
}
