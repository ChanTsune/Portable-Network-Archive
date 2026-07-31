use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive exists but requested entry path does not.
/// Action: Run `pna xattr set` with a non-existent entry path.
/// Expectation: The command returns an error and leaves the archive unchanged.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "xattr_missing_set/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "xattr_missing_set/archive.pna",
        "--overwrite",
        "xattr_missing_set/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let before = fs::read("xattr_missing_set/archive.pna").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_missing_set/archive.pna",
        "--name",
        "user.test",
        "--value",
        "test_value",
        "xattr_missing_set/in/raw/empty.txt",
        "xattr_missing_set/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(
        fs::read("xattr_missing_set/archive.pna").unwrap() == before,
        "the archive must be left unchanged"
    );
}
