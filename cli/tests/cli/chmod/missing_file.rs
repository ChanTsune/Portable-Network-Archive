use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive contains files, but one target file does not exist in the archive.
/// Action: Run `pna experimental chmod` targeting both existing and non-existing files.
/// Expectation: The command fails with an error and leaves the archive unchanged.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "chmod_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "chmod_missing/archive.pna",
        "--overwrite",
        "chmod_missing/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let before = fs::read("chmod_missing/archive.pna").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_missing/archive.pna",
        "600",
        "chmod_missing/in/raw/empty.txt",
        "chmod_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(
        fs::read("chmod_missing/archive.pna").unwrap() == before,
        "the archive must be left unchanged"
    );
}
