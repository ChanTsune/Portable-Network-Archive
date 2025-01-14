use crate::utils::{components_count, copy_dir_all, diff::diff, setup};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn archive_password_from_file() {
    setup();
    copy_dir_all(
        "../resources/test/raw",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/in/"
        ),
    )
    .unwrap();
    let password_file_path = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/archive_password_from_file/password_file"
    );
    fs::write(&password_file_path, "archive_password_from_file").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/password_from_file.pna"
        ),
        "--overwrite",
        "-r",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/in/"
        ),
        "--password-file",
        password_file_path,
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
            "/archive_password_from_file/password_from_file.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/out/"
        ),
        "--password",
        "archive_password_from_file",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/in/"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_password_from_file/out/"
        ),
    )
    .unwrap();
}
