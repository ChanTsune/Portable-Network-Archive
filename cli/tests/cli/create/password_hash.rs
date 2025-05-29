use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn aes_ctr_argon2_archive() {
    setup();
    TestResources::extract_in("raw/", "aes_argon2_ctr/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "aes_argon2_ctr/zstd_aes_argon2_ctr.pna",
        "--overwrite",
        "aes_argon2_ctr/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "aes_argon2_ctr/zstd_aes_argon2_ctr.pna",
        "--overwrite",
        "--out-dir",
        "aes_argon2_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("aes_argon2_ctr/in/", "aes_argon2_ctr/out/").unwrap();
}

#[test]
fn aes_ctr_argon2_with_params_archive() {
    setup();

    TestResources::extract_in("raw/", "aes_argon2_with_params_ctr/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "aes_argon2_with_params_ctr/zstd_aes_argon2_with_params_ctr.pna",
        "--overwrite",
        "aes_argon2_with_params_ctr/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=100,m=250,p=2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "aes_argon2_with_params_ctr/zstd_aes_argon2_with_params_ctr.pna",
        "--overwrite",
        "--out-dir",
        "aes_argon2_with_params_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "aes_argon2_with_params_ctr/in/",
        "aes_argon2_with_params_ctr/out/",
    )
    .unwrap();
}

#[test]
fn aes_ctr_pbkdf2_archive() {
    setup();
    TestResources::extract_in("raw/", "aes_pbkdf2_ctr/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "aes_pbkdf2_ctr/zstd_aes_pbkdf2_ctr.pna",
        "--overwrite",
        "aes_pbkdf2_ctr/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--pbkdf2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "aes_pbkdf2_ctr/zstd_aes_pbkdf2_ctr.pna",
        "--overwrite",
        "--out-dir",
        "aes_pbkdf2_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("aes_pbkdf2_ctr/in/", "aes_pbkdf2_ctr/out/").unwrap();
}

#[test]
fn aes_ctr_pbkdf2_with_params_archive() {
    setup();
    TestResources::extract_in("raw/", "aes_pbkdf2_with_params_ctr/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "aes_pbkdf2_with_params_ctr/zstd_aes_pbkdf2_with_params_ctr.pna",
        "--overwrite",
        "aes_pbkdf2_with_params_ctr/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--pbkdf2",
        "r=1",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "aes_pbkdf2_with_params_ctr/zstd_aes_pbkdf2_with_params_ctr.pna",
        "--overwrite",
        "--out-dir",
        "aes_pbkdf2_with_params_ctr/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "aes_pbkdf2_with_params_ctr/in/",
        "aes_pbkdf2_with_params_ctr/out/",
    )
    .unwrap();
}
