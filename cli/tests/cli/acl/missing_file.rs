use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive exists but one of the requested entry paths does not.
/// Action: Run `pna acl get` with both existing and non-existent entry paths.
/// Expectation: The command returns an error due to the missing entry.
#[test]
fn fail_with_missing_file_get() {
    setup();
    TestResources::extract_in("raw/", "acl_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "acl_missing/archive.pna",
        "--overwrite",
        "acl_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "get",
        "-f",
        "acl_missing/archive.pna",
        "acl_missing/in/raw/empty.txt",
        "acl_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}

/// Precondition: An archive exists but one of the requested entry paths does not.
/// Action: Run `pna acl set` with both existing and non-existent entry paths.
/// Expectation: The command returns an error due to the missing entry.
#[test]
fn fail_with_missing_file_set() {
    setup();
    TestResources::extract_in("raw/", "acl_missing_set/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "acl_missing_set/archive.pna",
        "--overwrite",
        "acl_missing_set/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_missing_set/archive.pna",
        "--set",
        "u::rwx",
        "acl_missing_set/in/raw/empty.txt",
        "acl_missing_set/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
