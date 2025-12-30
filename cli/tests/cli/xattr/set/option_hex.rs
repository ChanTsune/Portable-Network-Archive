use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive with multiple entries exists.
/// Action: Set an xattr with hex-encoded value (0x prefix).
/// Expectation: Target entry has the decoded value; other entries remain unaffected.
#[test]
fn xattr_set_hex() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set_hex/").unwrap();

    // Set hex encoded value
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_set_hex/zstd.pna",
        "--name",
        "user.hex",
        "--value",
        "0x48656c6c6f20576f726c64", // "Hello World" in hex
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify the value was set correctly
    archive::for_each_entry("xattr_set_hex/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.hex".into(),
                    b"Hello World".to_vec()
                )]
            );
        } else {
            // Non-target entries should remain unaffected (no xattrs)
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
            );
        }
    })
    .unwrap();
}
