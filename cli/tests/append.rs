use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_append() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "../resources/test/store.pna",
        "../resources/test/zstd.pna",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/append/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn archive_append_split() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/append_split.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--split",
        "100kib",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        &format!("{}/append_split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "../resources/test/store.pna",
        "../resources/test/zstd.pna",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/append_split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/append_split/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
