use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn create_with_password_file() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file/in/").unwrap();
    let password_file_path = "create_with_password_file/password_file";
    let password = "create_with_password_file";
    fs::write(password_file_path, password).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_password_file/password_from_file.pna",
        "--overwrite",
        "-r",
        "create_with_password_file/in/",
        "--password-file",
        password_file_path,
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_password_file/password_from_file.pna",
        "--overwrite",
        "--out-dir",
        "create_with_password_file/out/",
        "--password",
        password,
        "--strip-components",
        "2",
    ]))
    .unwrap();

    diff(
        "create_with_password_file/in/",
        "create_with_password_file/out/",
    )
    .unwrap();
}
