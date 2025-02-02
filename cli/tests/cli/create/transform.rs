use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn create_with_transform() {
    setup();
    TestResources::extract_in("raw/", "create_with_transform/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_transform/create_with_transform.pna",
        "--overwrite",
        "-r",
        "create_with_transform/in/",
        "--transform",
        "s,create_with_transform/in/,,",
        "--unstable",
    ]))
    .unwrap();
    assert!(fs::exists("create_with_transform/create_with_transform.pna").unwrap());

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_transform/create_with_transform.pna",
        "--overwrite",
        "--out-dir",
        "create_with_transform/out/",
    ]))
    .unwrap();

    diff("create_with_transform/in/", "create_with_transform/out/").unwrap();
}
