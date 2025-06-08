use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::io;

#[test]
fn test_update_non_existent_archive() -> io::Result<()> {
    setup();

    let args =
        cli::Cli::try_parse_from(["pna", "experimental", "update", "non_existent_archive.pna"])
            .unwrap();

    let result = args.execute();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);

    Ok(())
}
