use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn aes_ctr_argon2_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_argon2_ctr/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_ctr/zstd_aes_argon2_ctr.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_argon2_ctr/in/"),
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_ctr/zstd_aes_argon2_ctr.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_argon2_ctr/out/"),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_argon2_ctr/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_argon2_ctr/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_argon2_ctr/out/"),
    )
    .unwrap();
}

#[test]
fn aes_ctr_argon2_with_params_archive() {
    setup();

    TestResources::extract_in(
        "raw/",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/in/"
        ),
    )
    .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/zstd_aes_argon2_with_params_ctr.pna"
        ),
        "--overwrite",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/in/"
        ),
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=100,m=250,p=2",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/zstd_aes_argon2_with_params_ctr.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/out/"
        ),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/in/"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_argon2_with_params_ctr/out/"
        ),
    )
    .unwrap();
}

#[test]
fn aes_ctr_pbkdf2_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_pbkdf2_ctr/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_ctr/zstd_aes_pbkdf2_ctr.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_pbkdf2_ctr/in/"),
        "--password",
        "password",
        "--aes",
        "ctr",
        "--pbkdf2",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_ctr/zstd_aes_pbkdf2_ctr.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_pbkdf2_ctr/out/"),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_pbkdf2_ctr/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_pbkdf2_ctr/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/aes_pbkdf2_ctr/out/"),
    )
    .unwrap();
}

#[test]
fn aes_ctr_pbkdf2_with_params_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/in/"
        ),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/zstd_aes_pbkdf2_with_params_ctr.pna"
        ),
        "--overwrite",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/in/"
        ),
        "--password",
        "password",
        "--aes",
        "ctr",
        "--pbkdf2",
        "r=1",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/zstd_aes_pbkdf2_with_params_ctr.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/out/"
        ),
        "--password",
        "password",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/in/"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/aes_pbkdf2_with_params_ctr/out/"
        ),
    )
    .unwrap();
}
