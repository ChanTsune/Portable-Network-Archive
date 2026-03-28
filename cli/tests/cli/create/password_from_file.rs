use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

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

    let mut seen = HashSet::new();
    archive::for_each_entry_with_password(
        "create_with_password_file/password_from_file.pna",
        password,
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "create_with_password_file/in",
        "create_with_password_file/in/raw",
        "create_with_password_file/in/raw/empty.txt",
        "create_with_password_file/in/raw/text.txt",
        "create_with_password_file/in/raw/first",
        "create_with_password_file/in/raw/first/second",
        "create_with_password_file/in/raw/first/second/third",
        "create_with_password_file/in/raw/first/second/third/pna.txt",
        "create_with_password_file/in/raw/parent",
        "create_with_password_file/in/raw/parent/child.txt",
        "create_with_password_file/in/raw/images",
        "create_with_password_file/in/raw/images/icon.bmp",
        "create_with_password_file/in/raw/images/icon.png",
        "create_with_password_file/in/raw/images/icon.svg",
        "create_with_password_file/in/raw/pna",
        "create_with_password_file/in/raw/pna/empty.pna",
        "create_with_password_file/in/raw/pna/nest.pna",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
