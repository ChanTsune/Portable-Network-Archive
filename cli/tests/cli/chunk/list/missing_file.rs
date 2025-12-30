//! Tests for error handling when the archive file is missing.

use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: No archive file exists at the specified path.
/// Action: Run `pna experimental chunk list -f <nonexistent>`.
/// Expectation: Command fails with an appropriate error.
#[test]
fn chunk_list_missing_file() {
    setup();

    let result = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "chunk",
        "list",
        "-f",
        "nonexistent/archive.pna",
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "chunk list should fail for missing archive file"
    );
}
