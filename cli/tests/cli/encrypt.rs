use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn aes_ctr_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_ctr/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_aes_ctr/zstd_aes_ctr.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_ctr/in/"),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_aes_ctr/zstd_aes_ctr.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_ctr/out/"),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_ctr/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_ctr/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_ctr/out/"),
    )
    .unwrap();
}

#[test]
fn aes_cbc_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_cbc/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_aes_cbc/zstd_aes_cbc.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_cbc/in/"),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_aes_cbc/zstd_aes_cbc.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_cbc/out/"),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_cbc/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_cbc/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_aes_cbc/out/"),
    )
    .unwrap();
}

#[test]
fn camellia_ctr_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_ctr/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_camellia_ctr/zstd_camellia_ctr.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_ctr/in/"),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_camellia_ctr/zstd_camellia_ctr.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_ctr/out/"),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_camellia_ctr/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_ctr/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_ctr/out/"),
    )
    .unwrap();
}

#[test]
fn camellia_cbc_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_cbc/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_camellia_cbc/zstd_camellia_cbc.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_cbc/in/"),
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_camellia_cbc/zstd_camellia_cbc.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_cbc/out/"),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/zstd_camellia_cbc/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_cbc/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/zstd_camellia_cbc/out/"),
    )
    .unwrap();
}
