use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_with_substitution() {
    setup();
    TestResources::extract_in("raw/", "create_with_substitution/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_substitution/create_with_substitution.pna",
        "--overwrite",
        "create_with_substitution/in/",
        "-s",
        "#create_with_substitution/in/##",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("create_with_substitution/create_with_substitution.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_substitution/create_with_substitution.pna",
        "--overwrite",
        "--out-dir",
        "create_with_substitution/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "create_with_substitution/in/",
        "create_with_substitution/out/",
    )
    .unwrap();
}
