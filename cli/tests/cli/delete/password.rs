use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_password() {
    setup();
    TestResources::extract_in("raw/", "delete_password/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_password/delete_password.pna",
        "--overwrite",
        "delete_password/in/",
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
        "experimental",
        "delete",
        "delete_password/delete_password.pna",
        "**/raw/empty.txt",
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_password/in/raw/empty.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_password/delete_password.pna",
        "--overwrite",
        "--out-dir",
        "delete_password/out/",
        "--password",
        "password",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff("delete_password/in/", "delete_password/out/").unwrap();
}
