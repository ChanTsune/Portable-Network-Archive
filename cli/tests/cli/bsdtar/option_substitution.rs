use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: A single file is archived with a substitution rule that yields an empty pathname.
/// Action: Create and list archive via bsdtar with `-s ,in/d1/foo,,`.
/// Expectation: Entry is skipped (no blank pathname entry appears in list output).
#[test]
fn bsdtar_substitution_empty_name_is_skipped() {
    setup();
    fs::create_dir_all("bsdtar_substitution_empty_name_is_skipped/in/d1").unwrap();
    fs::write(
        "bsdtar_substitution_empty_name_is_skipped/in/d1/foo",
        b"foo",
    )
    .unwrap();

    let mut create = cargo_bin_cmd!("pna");
    create
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--unstable",
            "-c",
            "-f",
            "bsdtar_substitution_empty_name_is_skipped/archive.pna",
            "-C",
            "bsdtar_substitution_empty_name_is_skipped",
            "-s",
            ",in/d1/foo,,",
            "in/d1/foo",
        ])
        .assert()
        .success();

    let mut list = cargo_bin_cmd!("pna");
    list.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--unstable",
        "-t",
        "-f",
        "bsdtar_substitution_empty_name_is_skipped/archive.pna",
    ])
    .assert()
    .success()
    .stdout("");
}
