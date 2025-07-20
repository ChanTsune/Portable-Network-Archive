use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn fail_without_overwrite() {
    setup();
    TestResources::extract_in("raw/", "create_without_overwrite/src/").unwrap();
    let archive = "create_without_overwrite/create_without_overwrite.pna";
    fs::create_dir_all("create_without_overwrite").unwrap();
    fs::write(archive, b"exist").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        archive,
        "create_without_overwrite/src/",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
