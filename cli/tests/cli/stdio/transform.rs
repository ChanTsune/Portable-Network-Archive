use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn stdio_with_transform() {
    setup();
    TestResources::extract_in("raw/", "stdio_with_transform/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-c",
        "-f",
        "stdio_with_transform/archive.pna",
        "--overwrite",
        "--transform",
        "s,stdio_with_transform/in/,,",
        "stdio_with_transform/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        fs::exists("stdio_with_transform/archive.pna").unwrap(),
        "archive should be created"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "stdio",
        "-x",
        "-f",
        "stdio_with_transform/archive.pna",
        "--overwrite",
        "--out-dir",
        "stdio_with_transform/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("stdio_with_transform/in/", "stdio_with_transform/out/").unwrap();
}
