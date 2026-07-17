#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;

#[test]
fn bsdtar_block_size_option() {
    setup();
    let file = "bsdtar_block_size_option.txt";
    fs::write(file, "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--block-size")
        .arg("20")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Option '--block-size 20' is accepted for compatibility but will be ignored.",
        ));
}

#[test]
fn bsdtar_b_option() {
    setup();
    let file = "bsdtar_b_option.txt";
    fs::write(file, "").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("-b")
        .arg("20")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Option '--block-size 20' is accepted for compatibility but will be ignored.",
        ));
}
