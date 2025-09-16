use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn fail_with_missing_file() {
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
