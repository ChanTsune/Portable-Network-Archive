use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn extract_with_substitution() {
    setup();
    TestResources::extract_in("raw/", "extract_with_substitution/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_with_substitution/extract_with_substitution.pna",
        "--overwrite",
        "extract_with_substitution/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("extract_with_substitution/extract_with_substitution.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_substitution/extract_with_substitution.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_substitution/out/",
        "-s",
        "#extract_with_substitution/in/##",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "extract_with_substitution/in/",
        "extract_with_substitution/out/",
    )
    .unwrap();
}
