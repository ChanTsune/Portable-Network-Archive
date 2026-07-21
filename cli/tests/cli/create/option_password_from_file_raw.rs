use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: Input files and a password file whose content includes a trailing newline.
/// Action: Create an encrypted archive with `--password-file-raw`.
/// Expectation: The entire file content (including the newline) is used as the password.
#[test]
fn create_with_password_file_raw() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file_raw/in/").unwrap();
    let password_file_path = "create_with_password_file_raw/password_file";
    // Include a trailing newline so the raw password differs from a line-trimmed one.
    let password = "create_with_password_file_raw\n";
    fs::write(password_file_path, password).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_password_file_raw/password_from_file.pna",
        "--overwrite",
        "create_with_password_file_raw/in/",
        "--password-file-raw",
        password_file_path,
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry_with_password(
        "create_with_password_file_raw/password_from_file.pna",
        password,
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "create_with_password_file_raw/in",
        "create_with_password_file_raw/in/raw",
        "create_with_password_file_raw/in/raw/empty.txt",
        "create_with_password_file_raw/in/raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
}

/// Precondition: Input files and a password file whose content includes a trailing newline.
/// Action: Create an encrypted archive with `--password-file` (legacy full-file read).
/// Expectation: The entire file content (including the newline) is still accepted as the password.
#[test]
fn create_with_password_file_including_newline() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file_newline/in/").unwrap();
    let password_file_path = "create_with_password_file_newline/password_file";
    let password = "create_with_password_file_newline\n";
    fs::write(password_file_path, password).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_password_file_newline/password_from_file.pna",
        "--overwrite",
        "create_with_password_file_newline/in/",
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

    archive::for_each_entry_with_password(
        "create_with_password_file_newline/password_from_file.pna",
        password,
        |_entry| {},
    )
    .unwrap();
}

/// Precondition: Input files and a password file whose content is not valid UTF-8.
/// Action: Create an encrypted archive with `--password-file` (legacy full-file read).
/// Expectation: The entire byte sequence is still accepted as the password (only a warning is
/// emitted; the upcoming first-non-empty-UTF-8-line behavior would reject this file outright).
#[test]
fn create_with_password_file_non_utf8() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file_non_utf8/in/").unwrap();
    let password_file_path = "create_with_password_file_non_utf8/password_file";
    let password: &[u8] = &[0xff, 0xfe, 0xfd];
    fs::write(password_file_path, password).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_password_file_non_utf8/password_from_file.pna",
        "--overwrite",
        "create_with_password_file_non_utf8/in/",
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

    let mut archive =
        pna::Archive::open("create_with_password_file_non_utf8/password_from_file.pna").unwrap();
    let read_options = pna::ReadOptions::with_password(Some(password));
    for entry in archive.entries().extract_solid_entries(&read_options) {
        entry.unwrap();
    }
}
