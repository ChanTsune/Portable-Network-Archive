use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn create_with_substitution() {
    setup();
    TestResources::extract_in("raw/", "create_with_substitution/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_substitution/create_with_substitution.pna",
        "--overwrite",
        "-r",
        "create_with_substitution/in/",
        "-s",
        "#create_with_substitution/in/##",
        "--unstable",
    ]))
    .unwrap();
    assert!(fs::exists("create_with_substitution/create_with_substitution.pna").unwrap());

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_substitution/create_with_substitution.pna",
        "--overwrite",
        "--out-dir",
        "create_with_substitution/out/",
    ]))
    .unwrap();

    diff(
        "create_with_substitution/in/",
        "create_with_substitution/out/",
    )
    .unwrap();
}
