use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: No archive exists at the specified path.
/// Action: Run `pna experimental update` on a non-existent archive.
/// Expectation: Command returns an error.
#[test]
fn update_non_existent_archive() {
    setup();

    let args = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "update",
        "-f",
        "non_existent_archive.pna",
    ])
    .unwrap();

    let result = args.execute();

    assert!(result.is_err());
}
