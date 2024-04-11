use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn aes_ctr_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/zstd_aes_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--password",
        "password",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/zstd_aes_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &env!("CARGO_TARGET_TMPDIR"),
        "--password",
        "password",
    ]))
    .unwrap();
}
