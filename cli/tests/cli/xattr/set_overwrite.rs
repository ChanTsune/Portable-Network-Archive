use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive entry has an xattr set.
/// Action: Set the same xattr name with a different value.
/// Expectation: Target entry has the new value (overwritten); other entries remain unaffected.
#[test]
fn xattr_overwrite() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_overwrite/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        "xattr_overwrite/zstd.pna",
        "--name",
        "user.name",
        "--value",
        "first",
        "raw/empty.txt",
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
        "xattr_overwrite/zstd.pna",
        "--name",
        "user.name",
        "--value",
        "second",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_overwrite/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.name".into(),
                    b"second".into()
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
