use crate::utils::{archive::for_each_entry, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn archive_xattr_remove() {
    setup();
    TestResources::extract_in("raw/", "xattr_remove/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_remove/xattr_remove.pna",
        "--overwrite",
        "xattr_remove/in/",
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
        "xattr_remove/xattr_remove.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "xattr_remove/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    for_each_entry("xattr_remove/xattr_remove.pna", |entry| {
        if entry.header().path().as_str() == "xattr_remove/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.name".into(),
                    b"pna developers!".into()
                )],
            );
        }
    })
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_remove/xattr_remove.pna",
        "--remove",
        "user.name",
        "xattr_remove/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    for_each_entry("xattr_remove/xattr_remove.pna", |entry| {
        if entry.header().path().as_str() == "xattr_remove/in/raw/empty.txt" {
            assert!(entry.xattrs().is_empty());
        }
    })
    .unwrap();
}
