use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive contains files, but one target file does not exist in the archive.
/// Action: Run `pna experimental chown` targeting both existing and non-existing files.
/// Expectation: The command fails with an error and leaves the archive unchanged.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "chown_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "chown_missing/archive.pna",
        "--overwrite",
        "chown_missing/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let before = fs::read("chown_missing/archive.pna").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_missing/archive.pna",
        "test_user:test_group",
        "chown_missing/in/raw/empty.txt",
        "chown_missing/in/raw/not_found.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(
        fs::read("chown_missing/archive.pna").unwrap() == before,
        "the archive must be left unchanged"
    );
}
