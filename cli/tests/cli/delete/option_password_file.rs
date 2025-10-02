use crate::utils::{archive, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` with `--password` to build an encrypted archive, then delete entries
///          by `pna experimental delete` with `--password-file`
/// Expectation: Only the entries specified is removed from the encrypted archive; all
///         other entries remain.
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
        "-f",
        "delete_password_file/password_file.pna",
        "**/raw/empty.txt",
        "--password-file",
        password_file_path,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry_with_password(
        "delete_password_file/password_file.pna",
        password,
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "delete_password_file/in/raw/first/second/third/pna.txt",
        "delete_password_file/in/raw/images/icon.png",
        "delete_password_file/in/raw/images/icon.bmp",
        "delete_password_file/in/raw/images/icon.svg",
        "delete_password_file/in/raw/pna/empty.pna",
        "delete_password_file/in/raw/parent/child.txt",
        "delete_password_file/in/raw/pna/nest.pna",
        "delete_password_file/in/raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
