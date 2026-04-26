use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn aes_ctr_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_aes_ctr/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "zstd_aes_ctr/zstd_aes_ctr.pna",
        "--overwrite",
        "zstd_aes_ctr/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "zstd_aes_ctr/zstd_aes_ctr.pna",
        "--overwrite",
        "--out-dir",
        "zstd_aes_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_dirs_equal("zstd_aes_ctr/in/", "zstd_aes_ctr/out/");
}

#[test]
fn aes_cbc_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_aes_cbc/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "zstd_aes_cbc/zstd_aes_cbc.pna",
        "--overwrite",
        "zstd_aes_cbc/in/",
        "--password",
        "password",
        "--aes",
        "cbc",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "zstd_aes_cbc/zstd_aes_cbc.pna",
        "--overwrite",
        "--out-dir",
        "zstd_aes_cbc/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_dirs_equal("zstd_aes_cbc/in/", "zstd_aes_cbc/out/");
}

#[test]
fn camellia_ctr_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_camellia_ctr/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "zstd_camellia_ctr/zstd_camellia_ctr.pna",
        "--overwrite",
        "zstd_camellia_ctr/in/",
        "--password",
        "password",
        "--camellia",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "zstd_camellia_ctr/zstd_camellia_ctr.pna",
        "--overwrite",
        "--out-dir",
        "zstd_camellia_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_dirs_equal("zstd_camellia_ctr/in/", "zstd_camellia_ctr/out/");
}

#[test]
fn camellia_cbc_archive() {
    setup();
    TestResources::extract_in("raw/", "zstd_camellia_cbc/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "zstd_camellia_cbc/zstd_camellia_cbc.pna",
        "--overwrite",
        "zstd_camellia_cbc/in/",
        "--password",
        "password",
        "--aes",
        "cbc",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "zstd_camellia_cbc/zstd_camellia_cbc.pna",
        "--overwrite",
        "--out-dir",
        "zstd_camellia_cbc/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_dirs_equal("zstd_camellia_cbc/in/", "zstd_camellia_cbc/out/");
}
