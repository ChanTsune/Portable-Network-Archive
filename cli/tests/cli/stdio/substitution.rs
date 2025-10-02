use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn stdio_with_substitution() {
    setup();
    TestResources::extract_in("raw/", "stdio_with_substitution/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-c",
        "-f",
        "stdio_with_substitution/archive.pna",
        "--overwrite",
        "-s",
        "#stdio_with_substitution/in/##",
        "stdio_with_substitution/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        fs::exists("stdio_with_substitution/archive.pna").unwrap(),
        "archive should be created"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "-f",
        "stdio_with_substitution/archive.pna",
        "--overwrite",
        "--out-dir",
        "stdio_with_substitution/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "stdio_with_substitution/in/",
        "stdio_with_substitution/out/",
    )
    .unwrap();
}
