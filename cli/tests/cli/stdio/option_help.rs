#![cfg(not(target_family = "wasm"))]
use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: The stdio subcommand supports both -h and --help flags.
/// Action: Run stdio with --help, then with -h.
/// Expectation: Both produce identical output on stdout with no stderr.
#[test]
fn stdio_help_short_form_matches_long_help() {
    setup();

    let mut help_cmd = cargo_bin_cmd!("pna");
    let help = help_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .clone();

    let mut short_help_cmd = cargo_bin_cmd!("pna");
    let short_help = short_help_cmd
        .arg("experimental")
        .arg("stdio")
        .arg("-h")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(help.stderr.is_empty());
    assert!(short_help.stderr.is_empty());
    assert_eq!(short_help.stdout, help.stdout);
}
