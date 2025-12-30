use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive with multiple entries exists.
/// Action: Set an xattr with base64-encoded value (0s prefix).
/// Expectation: Target entry has the decoded value; other entries remain unaffected.
#[test]
fn xattr_set_base64() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set_base64/").unwrap();

    // Set base64 encoded value (must start with 0s)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_set_base64/zstd.pna",
        "--name",
        "user.base64",
        "--value",
        "0sSGVsbG8gV29ybGQ=", // "Hello World" in base64
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify the value was set correctly (decoded value)
    archive::for_each_entry("xattr_set_base64/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.base64".into(),
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
