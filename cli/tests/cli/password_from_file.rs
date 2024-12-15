use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;
use std::path::PathBuf;

#[test]
fn archive_password_from_file() {
    setup();
    let password_file_path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("password_file");
    fs::write(&password_file_path, "archive_password_from_file").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/password_from_file.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--password-file",
        password_file_path.to_str().unwrap(),
        "--aes",
        "ctr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/password_from_file.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/password_from_file/", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "archive_password_from_file",
    ]))
    .unwrap();
}
