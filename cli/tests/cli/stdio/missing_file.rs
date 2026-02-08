use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;

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
