use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
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

/// Precondition: Input files and a password file with a trailing newline after the password.
/// Action: Create an encrypted archive with `--password-file`.
/// Expectation: Only the first non-empty line is used (trailing newline is ignored).
#[test]
fn create_with_password_file_ignores_trailing_newline() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file_newline/in/").unwrap();
    let password_file_path = "create_with_password_file_newline/password_file";
    let password = "create_with_password_file_newline";
    fs::write(password_file_path, format!("{password}\n")).unwrap();
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

/// Precondition: Input files and a multi-line password file.
/// Action: Create an encrypted archive with `--password-file`.
/// Expectation: Only the first non-empty line is used as the password.
#[test]
fn create_with_password_file_uses_first_non_empty_line() {
    setup();
    TestResources::extract_in("raw/", "create_with_password_file_first_line/in/").unwrap();
    let password_file_path = "create_with_password_file_first_line/password_file";
    let password = "first-line-password";
    fs::write(
        password_file_path,
        format!("\n{password}\nsecond-line-ignored\n"),
    )
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_password_file_first_line/password_from_file.pna",
        "--overwrite",
        "create_with_password_file_first_line/in/",
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
        "create_with_password_file_first_line/password_from_file.pna",
        password,
        |_entry| {},
    )
    .unwrap();
}

/// Precondition: Input files and an empty password file.
/// Action: Create an encrypted archive with `--password-file`.
/// Expectation: The command fails because the password is empty.
#[test]
fn create_with_empty_password_file_is_rejected() {
    setup();
    TestResources::extract_in("raw/", "create_with_empty_password_file/in/").unwrap();
    let password_file_path = "create_with_empty_password_file/password_file";
    fs::write(password_file_path, "\n\n").unwrap();
    let err = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_empty_password_file/password_from_file.pna",
        "--overwrite",
        "create_with_empty_password_file/in/",
        "--password-file",
        password_file_path,
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    let err = format!("{err:?}");
    assert!(err.contains("password is empty"), "unexpected error: {err}");
}

/// Precondition: Input files and a password file whose content is not valid UTF-8.
/// Action: Create an encrypted archive with `--password-file`.
/// Expectation: The command fails because the password file must be valid UTF-8
/// (use `--password-file-raw` for arbitrary bytes).
#[test]
fn create_with_non_utf8_password_file_is_rejected() {
    setup();
    TestResources::extract_in("raw/", "create_with_non_utf8_password_file/in/").unwrap();
    let password_file_path = "create_with_non_utf8_password_file/password_file";
    fs::write(password_file_path, [0xff, 0xfe, 0xfd]).unwrap();
    let err = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_non_utf8_password_file/password_from_file.pna",
        "--overwrite",
        "create_with_non_utf8_password_file/in/",
        "--password-file",
        password_file_path,
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    let err = format!("{err:?}");
    assert!(err.contains("UTF-8"), "unexpected error: {err}");
}
