#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::{PredicateBooleanExt, predicate};
use std::fs;

/// Precondition: A file exists in the filesystem.
/// Action: Run `pna compat bsdtar` --create with `--options` specifying a global compression level.
/// Expectation: The archive is created successfully.
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

/// Precondition: A file exists in the filesystem.
/// Action: Run `pna compat bsdtar` --create with `--options` specifying a module-scoped compression level.
/// Expectation: The archive is created successfully.
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

/// Precondition: A file exists in the filesystem.
/// Action: Run `pna compat bsdtar` --create with a deprecated flag-level compression option.
/// Expectation: The command succeeds but emits a deprecation warning.
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

/// Precondition: A file exists in the filesystem.
/// Action: Run `pna compat bsdtar` --create with `--options` for compression level.
/// Expectation: The command succeeds without any deprecation warning.
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

/// Precondition: A file exists in the filesystem.
/// Action: Run `pna compat bsdtar` --create with an invalid `--options` value.
/// Expectation: The command fails with an error message that includes context about the invalid option.
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
