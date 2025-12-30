use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive entry has multiple extended attributes set.
/// Action: Remove one attribute using `--remove` option.
/// Expectation: Only the removed attribute is gone; other attributes remain.
#[test]
fn xattr_multiple_set_and_remove() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_multi/").unwrap();

    // Set multiple xattrs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_multi/zstd.pna",
        "--name",
        "user.a",
        "--value",
        "A",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_multi/zstd.pna",
        "--name",
        "user.b",
        "--value",
        "B",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Remove one xattr
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_multi/zstd.pna",
        "--remove",
        "user.a",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("xattr_multi/zstd.pna", |entry| {
        if entry.name() == "raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new("user.b".into(), b"B".into())]
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
