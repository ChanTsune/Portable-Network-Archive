#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::{PredicateBooleanExt, predicate};
use std::fs;

/// --options with global compression-level creates archive successfully.
#[test]
fn stdio_options_global_compression_level() {
    setup();
    let file = "stdio_options_global.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
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
fn stdio_options_module_compression_level() {
    setup();
    let file = "stdio_options_module.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--zstd")
        .arg("--options=zstd:compression-level=15")
        .arg("--")
        .arg(file)
        .assert()
        .success();
}

/// Flag-level compression (--zstd=N) shows deprecation warning.
#[test]
fn stdio_flag_level_shows_deprecation_warning() {
    setup();
    let file = "stdio_flag_level_deprecated.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--zstd=15")
        .arg("--")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "compression level in flags is deprecated",
        ));
}

/// --options without flag-level does not show deprecation warning.
#[test]
fn stdio_options_no_deprecation_warning() {
    setup();
    let file = "stdio_options_no_deprecation.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--zstd")
        .arg("--options=compression-level=15")
        .arg("--")
        .arg(file)
        .assert()
        .success()
        .stderr(predicate::str::contains("deprecated").not());
}

/// Invalid --options value shows error with context.
#[test]
fn stdio_options_invalid_value_shows_context() {
    setup();
    let file = "stdio_options_invalid.txt";
    fs::write(file, "test content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.arg("experimental")
        .arg("stdio")
        .arg("-c")
        .arg("--zstd")
        .arg("--options=zstd:compression-level=abc")
        .arg("--")
        .arg(file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("zstd:compression-level:"));
}
