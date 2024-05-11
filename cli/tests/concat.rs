use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn concat_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "create",
        &format!("{}/concat.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw/",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "split",
        &format!("{}/concat.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--max-size",
        "100kb",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "concat",
        &format!("{}/concatenated.pna", env!("CARGO_TARGET_TMPDIR")),
        &format!("{}/concat.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/concatenated.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/concatenated/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
