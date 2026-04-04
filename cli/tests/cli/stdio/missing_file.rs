use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::PathBuf;

/// Precondition: No input paths are provided.
/// Action: Run `pna experimental stdio -c -f ...` without positional paths.
/// Expectation: Command fails similarly to bsdtar's "missing file" handling.
#[test]
fn stdio_create_without_inputs_fails() {
    setup();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--unstable",
        "-c",
        "-f",
        "stdio_create_without_inputs_fails.pna",
    ]);
    cmd.assert().failure();
}

/// Precondition: A directory exists but no real input files are specified.
/// Action: Run bsdtar create mode with only -C directory-change options
///   and zero file operands.
/// Expectation: Command fails — directory changes alone do not constitute input.
#[test]
fn stdio_create_with_only_directory_changes_fails() {
    setup();

    let base = PathBuf::from("stdio_create_only_dir_changes");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--unstable",
            "-cf",
            base.join("out.pna").to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
        ])
        .assert()
        .failure();
}
