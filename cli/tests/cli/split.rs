use crate::utils::{components_count, copy_dir_all, diff::diff, setup};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn split_archive() {
    setup();
    copy_dir_all(
        "../resources/test/raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "create",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/split.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/in/"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "split",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/split.pna"),
        "--overwrite",
        "--max-size",
        "100kb",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/split/"),
    ]))
    .unwrap();

    // check split file size
    for entry in fs::read_dir(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/split_archive/split/"
    ))
    .unwrap()
    {
        assert!(fs::metadata(entry.unwrap().path()).unwrap().len() <= 100 * 1000);
    }

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/split_archive/split/split.part1.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/in/")).to_string(),
    ]))
    .unwrap();

    // check completely extracted
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/split_archive/out/"),
    )
    .unwrap();
}
