use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn multipart_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/multipart.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "../resources/test/multipart_test.txt",
        "--unstable",
        "--split",
        "110",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/multipart.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        env!("CARGO_TARGET_TMPDIR"),
    ]))
    .unwrap();
}
