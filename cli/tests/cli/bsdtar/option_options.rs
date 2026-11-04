#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;

/// --options with global compression-level creates archive successfully.
#[test]
fn bsdtar_options_global_compression_level() {
    setup();
    let file = "bsdtar_options_global.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--zstd")
        .arg("--options=compression-level=15")
        .arg("--")
        .arg(file)
        .assert()
        .success();
}

/// --options with module-specific compression-level creates archive successfully.
#[test]
fn bsdtar_options_module_compression_level() {
    setup();
    let file = "bsdtar_options_module.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--zstd")
        .arg("--options=zstd:compression-level=15")
        .arg("--")
        .arg(file)
        .assert()
        .success();
}

/// Invalid --options value shows error with context.
#[test]
fn bsdtar_options_invalid_value_shows_context() {
    setup();
    let file = "bsdtar_options_invalid.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("compat")
        .arg("bsdtar")
        .arg("-c")
        .arg("--zstd")
        .arg("--options=zstd:compression-level=abc")
        .arg("--")
        .arg(file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("zstd:compression-level:"));
}
