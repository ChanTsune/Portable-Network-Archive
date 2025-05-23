use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn aes_ctr_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_aes_ctr/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "zstd_aes_ctr/zstd_aes_ctr.pna",
        "--overwrite",
        "zstd_aes_ctr/in/",
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
        "zstd_aes_ctr/zstd_aes_ctr.pna",
        "--overwrite",
        "--out-dir",
        "zstd_aes_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        &components_count("zstd_aes_ctr/in/").to_string(),
    ]))
    .unwrap();

    diff("zstd_aes_ctr/in/", "zstd_aes_ctr/out/").unwrap();
}

#[test]
fn aes_cbc_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_aes_cbc/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "zstd_aes_cbc/zstd_aes_cbc.pna",
        "--overwrite",
        "zstd_aes_cbc/in/",
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
        "zstd_aes_cbc/zstd_aes_cbc.pna",
        "--overwrite",
        "--out-dir",
        "zstd_aes_cbc/out/",
        "--password",
        "password",
        "--strip-components",
        &components_count("zstd_aes_cbc/in/").to_string(),
    ]))
    .unwrap();

    diff("zstd_aes_cbc/in/", "zstd_aes_cbc/out/").unwrap();
}

#[test]
fn camellia_ctr_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_camellia_ctr/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "zstd_camellia_ctr/zstd_camellia_ctr.pna",
        "--overwrite",
        "zstd_camellia_ctr/in/",
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
        "zstd_camellia_ctr/zstd_camellia_ctr.pna",
        "--overwrite",
        "--out-dir",
        "zstd_camellia_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        &components_count("zstd_camellia_ctr/in/").to_string(),
    ]))
    .unwrap();

    diff("zstd_camellia_ctr/in/", "zstd_camellia_ctr/out/").unwrap();
}

#[test]
fn camellia_cbc_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_camellia_cbc/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "zstd_camellia_cbc/zstd_camellia_cbc.pna",
        "--overwrite",
        "zstd_camellia_cbc/in/",
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
        "zstd_camellia_cbc/zstd_camellia_cbc.pna",
        "--overwrite",
        "--out-dir",
        "zstd_camellia_cbc/out/",
        "--password",
        "password",
        "--strip-components",
        &components_count("zstd_camellia_cbc/in/").to_string(),
    ]))
    .unwrap();

    diff("zstd_camellia_cbc/in/", "zstd_camellia_cbc/out/").unwrap();
}
