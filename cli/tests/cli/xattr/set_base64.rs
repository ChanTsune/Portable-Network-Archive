use crate::utils::{archive, setup, TestResources};
use clap::Parser;
use portable_network_archive::cli;
use portable_network_archive::command::Command;

#[test]
fn xattr_set_base64() {
    setup();
    TestResources::extract_in("raw/", "xattr_set_base64/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_set_base64/xattr_set_base64.pna",
        "--overwrite",
        "xattr_set_base64/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set base64 encoded value (must start with 0s)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_set_base64/xattr_set_base64.pna",
        "--name",
        "user.base64",
        "--value",
        "0sSGVsbG8gV29ybGQ=", // "Hello World" in base64
        "xattr_set_base64/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify the value was set correctly (decoded value)
    archive::for_each_entry("xattr_set_base64/xattr_set_base64.pna", |entry| {
        if entry.header().path() == "xattr_set_base64/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.base64".into(),
                    b"Hello World".to_vec()
                )]
            );
        }
    })
    .unwrap();
}
