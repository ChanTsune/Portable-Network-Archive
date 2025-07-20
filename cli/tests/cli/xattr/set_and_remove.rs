use crate::utils::{archive, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn xattr_multiple_set_and_remove() {
    setup();
    TestResources::extract_in("raw/", "xattr_multi/in/").unwrap();

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_multi/xattr_multi.pna",
        "--overwrite",
        "xattr_multi/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set multiple xattrs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_multi/xattr_multi.pna",
        "--name",
        "user.a",
        "--value",
        "A",
        "xattr_multi/in/raw/empty.txt",
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
        "xattr_multi/xattr_multi.pna",
        "--name",
        "user.b",
        "--value",
        "B",
        "xattr_multi/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Remove one xattr
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_multi/xattr_multi.pna",
        "--remove",
        "user.a",
        "xattr_multi/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_multi/xattr_multi.pna", |entry| {
        if entry.header().path() == "xattr_multi/out/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new("user.b".into(), b"B".into())]
            );
        }
    })
    .unwrap();
}
