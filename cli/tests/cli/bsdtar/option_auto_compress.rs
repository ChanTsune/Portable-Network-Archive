#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;

#[test]
fn bsdtar_auto_compress_option() {
    setup();
    let file = "bsdtar_auto_compress_option.txt";
    fs::write(file, "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
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
fn bsdtar_a_option() {
    setup();
    let file = "bsdtar_a_option.txt";
    fs::write(file, "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("-a")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Option '--auto-compress' is accepted for compatibility but will be ignored.",
        ));
}
