use crate::utils::{self, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn extract_with_exclude() {
    setup();
    TestResources::extract_in("raw/", "extract_with_exclude/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_with_exclude/extract_with_exclude.pna",
        "--overwrite",
        "-r",
        "extract_with_exclude/in/",
    ]))
    .unwrap();
    assert!(fs::exists("extract_with_exclude/extract_with_exclude.pna").unwrap());

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_exclude/extract_with_exclude.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_exclude/out/",
        "--strip-components",
        "2",
        "--exclude",
        "**.txt",
        "--unstable",
    ]))
    .unwrap();

    // Remove files that are expected to be excluded from input for comparison
    let expected_to_be_excluded = [
        "extract_with_exclude/in/raw/first/second/third/pna.txt",
        "extract_with_exclude/in/raw/parent/child.txt",
        "extract_with_exclude/in/raw/empty.txt",
        "extract_with_exclude/in/raw/text.txt",
    ];
    for file in expected_to_be_excluded {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff("extract_with_exclude/in/", "extract_with_exclude/out/").unwrap();
}
