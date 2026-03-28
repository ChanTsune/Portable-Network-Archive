use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: The source tree contains files and directories.
/// Action: Run `pna create` to build an archive, then compare with `pna experimental diff`.
/// Expectation: No differences are detected.
#[test]
fn diff_to_current_dir() {
    setup();
    TestResources::extract_in("raw/", "diff/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "-f",
        "diff/diff.pna",
        "--overwrite",
        "diff/in/",
    ])
    .assert()
    .success();

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args(["experimental", "diff", "-f", "diff/diff.pna"])
        .assert();

    assert.stdout("");
}
