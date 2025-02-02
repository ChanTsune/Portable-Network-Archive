use crate::utils::{self, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn create_with_exclude() {
    setup();
    TestResources::extract_in("raw/", "create_with_exclude/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_exclude/create_with_exclude.pna",
        "--overwrite",
        "-r",
        "create_with_exclude/in/",
        "--exclude",
        "**.txt",
        "--unstable",
    ]))
    .unwrap();
    assert!(fs::exists("create_with_exclude/create_with_exclude.pna").unwrap());

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_exclude/create_with_exclude.pna",
        "--overwrite",
        "--out-dir",
        "create_with_exclude/out/",
        "--strip-components",
        "2",
    ]))
    .unwrap();

    // Remove files that are expected to be excluded from input for comparison
    let expected_to_be_excluded = [
        "create_with_exclude/in/raw/first/second/third/pna.txt",
        "create_with_exclude/in/raw/parent/child.txt",
        "create_with_exclude/in/raw/empty.txt",
        "create_with_exclude/in/raw/text.txt",
    ];
    for file in expected_to_be_excluded {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff("create_with_exclude/in/", "create_with_exclude/out/").unwrap();
}
