use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_append() -> anyhow::Result<()> {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))?;
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "../resources/test/store.pna",
        "../resources/test/zstd.pna",
    ]))?;
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/append.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/append/", env!("CARGO_TARGET_TMPDIR")),
    ]))
}

#[test]
fn archive_append_split() -> anyhow::Result<()> {
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
    ]))?;
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "append",
        &format!("{}/append_split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "../resources/test/store.pna",
        "../resources/test/zstd.pna",
    ]))?;
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/append_split.part1.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/append_split/", env!("CARGO_TARGET_TMPDIR")),
    ]))
}
