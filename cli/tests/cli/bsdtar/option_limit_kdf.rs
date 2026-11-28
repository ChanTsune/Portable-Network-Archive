use crate::utils::{create_high_cost_encrypted_solid_archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

const PASSWORD: &str = "testpass";

fn prepare(name: &str) -> (String, String) {
    let dir = format!("bsdtar_kdf_limit_{name}");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let archive = format!("{dir}/source.pna");
    create_high_cost_encrypted_solid_archive(&archive, PASSWORD, "secret.txt", b"secret content");
    (dir, archive)
}

/// Precondition: An encrypted solid archive uses KDF parallelism above the bounded cap.
/// Action: List it with bsdtar without a KDF cost limit flag.
/// Expectation: Listing succeeds.
#[test]
fn bsdtar_list_default_accepts_high_cost_archive() {
    setup();
    let (_, archive) = prepare("list_default");

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "-t",
            "-f",
            &archive,
            "--password",
            PASSWORD,
        ])
        .assert()
        .success();
}

/// Precondition: An encrypted solid archive uses KDF parallelism above the bounded cap.
/// Action: List it with bsdtar with `--limit-kdf`.
/// Expectation: Listing fails because the KDF cost exceeds the cap.
#[test]
fn bsdtar_list_with_limit_kdf_rejects_high_cost_archive() {
    setup();
    let (_, archive) = prepare("list_bounded");

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "-t",
            "-f",
            &archive,
            "--password",
            PASSWORD,
            "--limit-kdf",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds maximum"));
}

/// Precondition: A source archive uses KDF parallelism above the bounded cap.
/// Action: Copy it into a new archive with bsdtar create using `--limit-kdf`.
/// Expectation: The command fails on the archive-source read path.
#[test]
fn bsdtar_create_from_archive_with_limit_kdf_rejects_high_cost_archive() {
    setup();
    let (dir, _) = prepare("create_bounded");

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "-c",
            "-f",
            &format!("{dir}/out.pna"),
            "-C",
            &dir,
            "--password",
            PASSWORD,
            "--limit-kdf",
            "@source.pna",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds maximum"));
}
