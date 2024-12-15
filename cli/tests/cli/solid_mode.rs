use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn solid_store_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/solid_store.pna", env!("CARGO_TARGET_TMPDIR")),
        "--store",
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/solid_store.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/solid_store/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn solid_zstd_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/solid_zstd.pna", env!("CARGO_TARGET_TMPDIR")),
        "--zstd",
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/solid_zstd.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/solid_zstd/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn solid_xz_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/solid_xz.pna", env!("CARGO_TARGET_TMPDIR")),
        "--xz",
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/solid_xz.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/solid_xz/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn solid_deflate_archive() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/solid_deflate.pna", env!("CARGO_TARGET_TMPDIR")),
        "--deflate",
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/solid_deflate.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/solid_deflate/", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}
