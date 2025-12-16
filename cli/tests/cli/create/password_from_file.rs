use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn create_with_password_file() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file/in/").unwrap();
    let password_file_path = "create_with_password_file/password_file";
    let password = "create_with_password_file";
    fs::write(password_file_path, password).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_password_file/password_from_file.pna",
        "--overwrite",
        "create_with_password_file/in/",
        "--password-file",
        password_file_path,
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
        "create_with_password_file/password_from_file.pna",
        "--overwrite",
        "--out-dir",
        "create_with_password_file/out/",
        "--password",
        password,
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "create_with_password_file/in/",
        "create_with_password_file/out/",
    )
    .unwrap();
}
