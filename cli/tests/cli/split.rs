use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn split_archive() {
    setup();
    TestResources::extract_in("raw/", "split_archive/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "create",
        "split_archive/split.pna",
        "--overwrite",
        "split_archive/in/",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "split",
        "split_archive/split.pna",
        "--overwrite",
        "--max-size",
        "100kb",
        "--out-dir",
        "split_archive/split/",
    ]))
    .unwrap();

    // check split file size
    for entry in fs::read_dir("split_archive/split/").unwrap() {
        assert!(fs::metadata(entry.unwrap().path()).unwrap().len() <= 100 * 1000);
    }

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "split_archive/split/split.part1.pna",
        "--overwrite",
        "--out-dir",
        "split_archive/out/",
        "--strip-components",
        &components_count("split_archive/in/").to_string(),
    ]))
    .unwrap();

    // check completely extracted
    diff("split_archive/in/", "split_archive/out/").unwrap();
}
