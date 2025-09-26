use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn extract_with_password_file() {
    setup();
    TestResources::extract_in("raw/", "extract_with_password_file/in/").unwrap();
    let password_file_path = "extract_with_password_file/password_file";
    let password = "extract_with_password_file";
    fs::write(password_file_path, password).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_with_password_file/password_from_file.pna",
        "--overwrite",
        "extract_with_password_file/in/",
        "--password",
        password,
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_password_file/password_from_file.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_password_file/out/",
        "--password-file",
        password_file_path,
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "extract_with_password_file/in/",
        "extract_with_password_file/out/",
    )
    .unwrap();
}
