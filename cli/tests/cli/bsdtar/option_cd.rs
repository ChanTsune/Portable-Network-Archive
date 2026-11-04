use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: Source tree is available for bsdtar archive creation.
/// Action: Create an archive via `pna compat bsdtar -c -C <input> .`,
///         then extract it via stdin with `pna compat bsdtar -x -C . --out-dir <out>`.
/// Expectation: Directory changes are applied before resolving create inputs and extract output.
#[test]
fn bsdtar_create_extract_with_cd() {
    setup();
    TestResources::extract_in("raw/", "bsdtar_with_cd/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "-c",
        "-C",
        "bsdtar_with_cd/in/",
        ".",
    ]);
    let assert = cmd.assert().success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(assert.get_output().stdout.as_slice());
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "-x",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "bsdtar_with_cd/out/",
    ]);
    cmd.assert().success();

    assert_dirs_equal("bsdtar_with_cd/in/", "bsdtar_with_cd/out/");
}
