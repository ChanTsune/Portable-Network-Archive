use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn aes_ctr_argon2_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/zstd_aes_argon2_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        &format!("{}/zstd_aes_argon2_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/zstd_aes_argon2_ctr/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn aes_ctr_argon2_with_params_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!(
            "{}/zstd_aes_argon2_with_params_ctr.pna",
            env!("CARGO_TARGET_TMPDIR")
        ),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        &format!(
            "{}/zstd_aes_argon2_with_params_ctr.pna",
            env!("CARGO_TARGET_TMPDIR")
        ),
        "--overwrite",
        "--out-dir",
        &format!(
            "{}/zstd_aes_argon2_with_params_ctr/",
            env!("CARGO_TARGET_TMPDIR")
        ),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn aes_ctr_pbkdf2_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/zstd_aes_pbkdf2_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        &format!("{}/zstd_aes_pbkdf2_ctr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/zstd_aes_pbkdf2_ctr/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn aes_ctr_pbkdf2_with_params_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!(
            "{}/zstd_aes_pbkdf2_with_params_ctr.pna",
            env!("CARGO_TARGET_TMPDIR")
        ),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        &format!(
            "{}/zstd_aes_pbkdf2_with_params_ctr.pna",
            env!("CARGO_TARGET_TMPDIR")
        ),
        "--overwrite",
        "--out-dir",
        &format!(
            "{}/zstd_aes_pbkdf2_with_params_ctr/",
            env!("CARGO_TARGET_TMPDIR")
        ),
        "--password",
        "password",
    ]))
    .unwrap();
}
