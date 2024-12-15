use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn aes_ctr_archive() {
    setup();
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
        "--aes",
        "ctr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/zstd_aes_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/zstd_aes_ctr/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn aes_cbc_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/zstd_aes_cbc.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--password",
        "password",
        "--aes",
        "cbc",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/zstd_aes_cbc.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/zstd_aes_cbc/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn camellia_ctr_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/zstd_camellia_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--password",
        "password",
        "--camellia",
        "ctr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/zstd_camellia_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/zstd_camellia_ctr/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn camellia_cbc_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/zstd_camellia_cbc.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--password",
        "password",
        "--aes",
        "cbc",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/zstd_camellia_cbc.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/zstd_camellia_cbc/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}
