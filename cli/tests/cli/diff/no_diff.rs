use crate::utils::{setup, EmbedExt, TestResources};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: the source tree contains file and directory.
/// Action: run `pna create` to build an archive, then compare by `pna experimental diff`.
/// Expectation: no difference detected.
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
