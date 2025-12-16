use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna experimental delete` targeting an existing entry and a missing entry.
/// Expectation: The command returns an error because at least one requested path is absent.
#[test]
fn delete_fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "delete_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_missing/archive.pna",
        "--overwrite",
        "delete_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_missing/archive.pna",
        "delete_missing/in/raw/empty.txt",
        "delete_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
