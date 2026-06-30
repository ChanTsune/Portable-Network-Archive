use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: Source tree is available for stdio archive creation.
/// Action: Create an archive via `pna experimental stdio -c -C <input> .`,
///         then extract it via stdin with `pna experimental stdio -x -C . --out-dir <out>`.
/// Expectation: Directory changes are applied before resolving create inputs and extract output.
#[test]
fn stdio_create_extract_with_cd() {
    setup();
    TestResources::extract_in("raw/", "stdio_with_cd/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "-c",
        "-C",
        "stdio_with_cd/in/",
        ".",
    ]);
    let assert = cmd.assert().success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(assert.get_output().stdout.as_slice());
    cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "stdio_with_cd/out/",
    ]);
    cmd.assert().success();

    assert_dirs_equal("stdio_with_cd/in/", "stdio_with_cd/out/");
}
