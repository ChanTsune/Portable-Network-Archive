use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive exists but one of the requested entry paths does not.
/// Action: Run `pna acl set` with both existing and non-existent entry paths.
/// Expectation: The command returns an error and leaves the archive unchanged.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "acl_set_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "acl_set_missing/archive.pna",
        "--overwrite",
        "acl_set_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let before = fs::read("acl_set_missing/archive.pna").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "--overwrite",
        "-f",
        "acl_set_missing/archive.pna",
        "--set",
        "u::rwx",
        "acl_set_missing/in/raw/empty.txt",
        "acl_set_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(
        fs::read("acl_set_missing/archive.pna").unwrap() == before,
        "the archive must be left unchanged"
    );
}
