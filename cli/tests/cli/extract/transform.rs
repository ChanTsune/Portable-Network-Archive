use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn extract_with_transform() {
    setup();
    TestResources::extract_in("raw/", "extract_with_transform/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_with_transform/extract_with_transform.pna",
        "--overwrite",
        "extract_with_transform/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("extract_with_transform/extract_with_transform.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_transform/extract_with_transform.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_transform/out/",
        "--transform",
        "s,extract_with_transform/in/,,",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("extract_with_transform/in/", "extract_with_transform/out/").unwrap();
}
