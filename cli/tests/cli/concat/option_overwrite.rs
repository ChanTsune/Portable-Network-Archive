use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

/// Precondition: Output file already exists.
/// Action: Run concat with --overwrite flag.
/// Expectation: Command succeeds and overwrites the existing file.
#[test]
fn concat_with_overwrite_succeeds() {
    setup();
    TestResources::extract_in("zstd.pna", "concat_overwrite_yes/").unwrap();

    // Create a dummy output file that will be overwritten
    fs::write("concat_overwrite_yes/output.pna", b"dummy content").unwrap();
    assert!(fs::metadata("concat_overwrite_yes/output.pna").is_ok());

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "concat",
        "--overwrite",
        "-f",
        "concat_overwrite_yes/output.pna",
        "-f",
        "concat_overwrite_yes/zstd.pna",
    ])
    .assert()
    .success();

    // Verify output is a valid PNA archive with at least one entry
    let mut count = 0usize;
    archive::for_each_entry("concat_overwrite_yes/output.pna", |_| {
        count += 1;
    })
    .unwrap();
    assert!(count > 0, "concatenated archive should contain entries");
}

/// Precondition: Output file already exists.
/// Action: Run concat without --overwrite flag.
/// Expectation: Command fails with "already exists" error.
#[test]
fn concat_without_overwrite_fails() {
    setup();
    TestResources::extract_in("zstd.pna", "concat_overwrite_no/").unwrap();

    // Create output file that should block the operation
    fs::write("concat_overwrite_no/output.pna", b"existing content").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "concat",
        "-f",
        "concat_overwrite_no/output.pna",
        "-f",
        "concat_overwrite_no/zstd.pna",
    ])
    .assert()
    .failure();
}
