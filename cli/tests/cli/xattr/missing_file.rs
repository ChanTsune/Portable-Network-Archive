use crate::utils::{setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn fail_with_missing_file_get() {
    setup();
    TestResources::extract_in("raw/", "xattr_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_missing/archive.pna",
        "--overwrite",
        "xattr_missing/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        "xattr_missing/archive.pna",
        "xattr_missing/in/raw/empty.txt",
        "xattr_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}

#[test]
fn fail_with_missing_file_set() {
    setup();
    TestResources::extract_in("raw/", "xattr_missing_set/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_missing_set/archive.pna",
        "--overwrite",
        "xattr_missing_set/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_missing_set/archive.pna",
        "--name",
        "user.test",
        "--value",
        "test_value",
        "xattr_missing_set/in/raw/empty.txt",
        "xattr_missing_set/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}
