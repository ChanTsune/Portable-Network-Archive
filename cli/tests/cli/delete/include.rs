use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_include() {
    setup();
    TestResources::extract_in("raw/", "delete_with_include/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_with_include/include.pna",
        "--overwrite",
        "delete_with_include/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_with_include/include.pna",
        "**/*.txt",
        "--include",
        "**/raw/text.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_with_include/in/raw/text.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_with_include/include.pna",
        "--overwrite",
        "--out-dir",
        "delete_with_include/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("delete_with_include/in/", "delete_with_include/out/").unwrap();
}
