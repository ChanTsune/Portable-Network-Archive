use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_password_file() {
    setup();
    TestResources::extract_in("raw/", "delete_password_file/in/").unwrap();
    let password_file_path = "delete_password_file/password_file";
    let password = "delete_password_file";
    fs::write(password_file_path, password).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_password_file/password_file.pna",
        "--overwrite",
        "delete_password_file/in/",
        "--password",
        password,
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_password_file/password_file.pna",
        "**/raw/empty.txt",
        "--password-file",
        password_file_path,
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_password_file/in/raw/empty.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_password_file/password_file.pna",
        "--overwrite",
        "--out-dir",
        "delete_password_file/out/",
        "--password-file",
        password_file_path,
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff("delete_password_file/in/", "delete_password_file/out/").unwrap();
}
