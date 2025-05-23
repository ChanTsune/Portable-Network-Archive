mod exclude;

use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_append() {
    setup();
    TestResources::extract_in("raw/", "archive_append/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append/append.pna",
        "--overwrite",
        "archive_append/in/",
    ]))
    .unwrap();

    // Copy extra input
    TestResources::extract_in("store.pna", "archive_append/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append/in/").unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        "archive_append/append.pna",
        "archive_append/in/store.pna",
        "archive_append/in/zstd.pna",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_append/append.pna",
        "--overwrite",
        "--out-dir",
        "archive_append/out/",
        "--strip-components",
        &components_count("archive_append/in/").to_string(),
    ]))
    .unwrap();
    // check completely extracted
    diff("archive_append/in/", "archive_append/out/").unwrap();
}

#[test]
fn archive_append_split() {
    setup();
    TestResources::extract_in("raw/", "archive_append_split/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_split/append_split.pna",
        "--overwrite",
        "archive_append_split/in/",
        "--split",
        "100kib",
    ]))
    .unwrap();

    // Copy extra input
    TestResources::extract_in("store.pna", "archive_append_split/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_split/in/").unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        "archive_append_split/append_split.part1.pna",
        "archive_append_split/in/store.pna",
        "archive_append_split/in/zstd.pna",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_append_split/append_split.part1.pna",
        "--overwrite",
        "--out-dir",
        "archive_append_split/out/",
        "--strip-components",
        &components_count("archive_append_split/out/").to_string(),
    ]))
    .unwrap();
    // check completely extracted
    diff("archive_append_split/in/", "archive_append_split/out/").unwrap();
}
