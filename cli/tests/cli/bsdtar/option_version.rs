#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: The bsdtar subcommand is configured with bsdtar-compatible version output.
/// Action: Run bsdtar with --version.
/// Expectation: Output starts with "bsdtar" and identifies portable-network-archive.
#[test]
fn bsdtar_version_uses_bsdtar_compatible_prefix() {
    setup();

    let mut cmd = cargo_bin_cmd!("pna");
    let output = cmd
        .arg("compat")
        .arg("bsdtar")
        .arg("--version")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("bsdtar "));
    assert!(stdout.contains(" - portable-network-archive "));
}
