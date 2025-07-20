use crate::utils::{archive, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn archive_xattr_set() {
    setup();
    TestResources::extract_in("raw/", "xattr_set/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_set/xattr_set.pna",
        "--overwrite",
        "xattr_set/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_set/xattr_set.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "xattr_set/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_set/xattr_set.pna", |entry| {
        if entry.header().path().as_str() == "xattr_set/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.name".into(),
                    b"pna developers!".into()
                )]
            );
        }
    })
    .unwrap();
}

#[test]
fn xattr_long_key_value() {
    setup();
    TestResources::extract_in("raw/", "xattr_long/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_long/xattr_long.pna",
        "--overwrite",
        "xattr_long/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let long_name = "user.".to_owned() + &"n".repeat(200);
    let long_value = "v".repeat(1024);
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_long/xattr_long.pna",
        "--name",
        &long_name,
        "--value",
        &long_value,
        "xattr_long/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_long/xattr_long.pna",
        "--name",
        "user.special",
        "--value",
        "\0\n\r\x7f\u{1F600}",
        "xattr_long/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_long/xattr_long.pna", |entry| {
        if entry.header().path() == "" {
            assert_eq!(
                entry.xattrs(),
                &[
                    pna::ExtendedAttribute::new(
                        long_name.as_str().into(),
                        long_value.as_bytes().into()
                    ),
                    pna::ExtendedAttribute::new(
                        "user.special".into(),
                        "\0\n\r\x7f\u{1F600}".into()
                    ),
                ]
            );
        }
    })
    .unwrap();
}

#[test]
fn xattr_empty_key() {
    setup();
    TestResources::extract_in("raw/", "xattr_empty_key/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_empty_key/xattr_empty_key.pna",
        "--overwrite",
        "xattr_empty_key/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_empty_key/xattr_empty_key.pna",
        "--name",
        "",
        "--value",
        "value",
        "xattr_empty_key/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_empty_key/xattr_empty_key.pna", |entry| {
        if entry.header().path() == "xattr_empty_key/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new("".into(), b"value".into())]
            );
        }
    })
    .unwrap();
}

#[test]
fn xattr_empty_value() {
    setup();
    TestResources::extract_in("raw/", "xattr_empty_value/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_empty_value/xattr_empty_value.pna",
        "--overwrite",
        "xattr_empty_value/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_empty_value/xattr_empty_value.pna",
        "--name",
        "user.empty",
        "--value",
        "",
        "xattr_empty_value/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_empty_value/xattr_empty_value.pna", |entry| {
        if entry.header().path() == "xattr_empty_value/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new("user.empty".into(), b"".into())]
            );
        }
    })
    .unwrap();
}
