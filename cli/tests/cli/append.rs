mod exclude;

use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_append() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/append.pna"),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/"),
    ]))
    .unwrap();

    // Copy extra input
    TestResources::extract_in(
        "store.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/"),
    )
    .unwrap();
    TestResources::extract_in(
        "zstd.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/"),
    )
    .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/append.pna"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/store.pna"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/zstd.pna"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/append.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/")).to_string(),
    ]))
    .unwrap();
    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append/out/"),
    )
    .unwrap();
}

#[test]
fn archive_append_split() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_append_split/append_split.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/in/"),
        "--split",
        "100kib",
    ]))
    .unwrap();

    // Copy extra input
    TestResources::extract_in(
        "store.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/in/"),
    )
    .unwrap();
    TestResources::extract_in(
        "zstd.pna",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/in/"),
    )
    .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_append_split/append_split.part1.pna"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_append_split/in/store.pna"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_append_split/in/zstd.pna"
        ),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_append_split/append_split.part1.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/out/"),
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_append_split/out/"
        ))
        .to_string(),
    ]))
    .unwrap();
    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_append_split/out/"),
    )
    .unwrap();
}
