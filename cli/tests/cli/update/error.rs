use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn test_update_non_existent_archive() {
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
