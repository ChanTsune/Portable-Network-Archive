use crate::utils::{archive, create_high_cost_encrypted_solid_archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

const PASSWORD: &str = "testpass";

fn prepare(name: &str) -> (String, String) {
    let dir = format!("delete_kdf_limit_{name}");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let target = format!("{dir}/target.pna");
    let reference = format!("{dir}/reference.pna");
    create_high_cost_encrypted_solid_archive(&target, PASSWORD, "secret.txt", b"secret content");
    fs::copy(&target, &reference).unwrap();
    (target, reference)
}

/// Precondition: An encrypted archive uses KDF parallelism above the bounded cap.
/// Action: Delete an entry from it without a KDF cost limit flag.
/// Expectation: The rewrite succeeds and the entry is removed.
#[test]
fn delete_default_accepts_high_cost_archive() {
    setup();
    let (target, _) = prepare("default");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet", "delete", "-f", &target, "secret.txt", "--password", PASSWORD,
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    archive::for_each_entry_with_password(&target, PASSWORD, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    assert!(names.is_empty(), "unexpected entries: {names:?}");
}

/// Precondition: An encrypted archive uses KDF parallelism above the bounded cap.
/// Action: Delete an entry from it with `--limit-kdf`.
/// Expectation: The rewrite fails and the original archive is left unchanged.
#[test]
fn delete_with_limit_kdf_rejects_high_cost_archive() {
    setup();
    let (target, reference) = prepare("bounded");

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "delete",
            "-f",
            &target,
            "secret.txt",
            "--password",
            PASSWORD,
            "--limit-kdf",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds maximum"));

    assert_eq!(fs::read(&target).unwrap(), fs::read(reference).unwrap());
}
