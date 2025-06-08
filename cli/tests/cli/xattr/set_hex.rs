use crate::utils::{archive, setup, TestResources};
use clap::Parser;
use portable_network_archive::cli;
use portable_network_archive::command::Command;

#[test]
fn xattr_set_hex() {
    setup();
    TestResources::extract_in("raw/", "xattr_set_hex/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_set_hex/xattr_set_hex.pna",
        "--overwrite",
        "xattr_set_hex/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set hex encoded value
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_set_hex/xattr_set_hex.pna",
        "--name",
        "user.hex",
        "--value",
        "0x48656c6c6f20576f726c64", // "Hello World" in hex
        "xattr_set_hex/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify the value was set correctly
    archive::for_each_entry("xattr_set_hex/xattr_set_hex.pna", |entry| {
        if entry.header().path() == "xattr_set_hex/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.hex".into(),
                    b"Hello World".to_vec()
                )]
            );
        }
    })
    .unwrap();
}
