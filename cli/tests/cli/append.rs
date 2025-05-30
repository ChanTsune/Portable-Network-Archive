mod atime;
mod ctime;
mod exclude;
mod mtime;

use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn archive_append() {
    setup();
    TestResources::extract_in("raw/", "archive_append/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append/append.pna",
        "--overwrite",
        "archive_append/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input
    TestResources::extract_in("store.pna", "archive_append/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "archive_append/append.pna",
        "archive_append/in/store.pna",
        "archive_append/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_append/append.pna",
        "--overwrite",
        "--out-dir",
        "archive_append/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // check completely extracted
    diff("archive_append/in/", "archive_append/out/").unwrap();
}

#[test]
fn archive_append_split() {
    setup();
    TestResources::extract_in("raw/", "archive_append_split/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_append_split/append_split.pna",
        "--overwrite",
        "archive_append_split/in/",
        "--split",
        "100kib",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input
    TestResources::extract_in("store.pna", "archive_append_split/in/").unwrap();
    TestResources::extract_in("zstd.pna", "archive_append_split/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "archive_append_split/append_split.part1.pna",
        "archive_append_split/in/store.pna",
        "archive_append_split/in/zstd.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_append_split/append_split.part1.pna",
        "--overwrite",
        "--out-dir",
        "archive_append_split/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // check completely extracted
    diff("archive_append_split/in/", "archive_append_split/out/").unwrap();
}
