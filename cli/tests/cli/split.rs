use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn split_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "create",
        &format!("{}/split.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw/",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "split",
        &format!("{}/split.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--max-size",
        "100kb",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/split/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
