#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;

#[test]
fn stdio_auto_compress_option() {
    setup();
    let file = "stdio_auto_compress_option.txt";
    fs::write(file, "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--auto-compress")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Option '--auto-compress' is accepted for compatibility but will be ignored.",
        ));
}

#[test]
fn stdio_a_option() {
    setup();
    let file = "stdio_a_option.txt";
    fs::write(file, "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("-a")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Option '--auto-compress' is accepted for compatibility but will be ignored.",
        ));
}
