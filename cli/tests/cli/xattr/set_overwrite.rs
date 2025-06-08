use crate::utils::{archive, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn xattr_overwrite() {
    setup();
    TestResources::extract_in("raw/", "xattr_overwrite/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_overwrite/xattr_overwrite.pna",
        "--overwrite",
        "xattr_overwrite/in/",
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
        "xattr_overwrite/xattr_overwrite.pna",
        "--name",
        "user.name",
        "--value",
        "first",
        "xattr_overwrite/in/raw/empty.txt",
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
        "xattr_overwrite/xattr_overwrite.pna",
        "--name",
        "user.name",
        "--value",
        "second",
        "xattr_overwrite/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_overwrite/xattr_overwrite.pna", |entry| {
        if entry.header().path() == "xattr_overwrite/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.name".into(),
                    b"second".into()
                )]
            );
        }
    })
    .unwrap();
}
