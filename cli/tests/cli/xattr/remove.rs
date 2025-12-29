use crate::utils::{EmbedExt, TestResources, archive::for_each_entry, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive entry has an extended attribute set.
/// Action: Remove the xattr using `--remove` option.
/// Expectation: The xattr is removed from the target entry; other entries remain unaffected.
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
        if entry.name() == "xattr_remove/in/raw/empty.txt" {
            assert_eq!(
                entry.xattrs(),
                &[pna::ExtendedAttribute::new(
                    "user.name".into(),
                    b"pna developers!".into()
                )],
            );
        } else {
            // Non-target entries should have no xattrs
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
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
        // After removal, all entries should have no xattrs
        assert!(
            entry.xattrs().is_empty(),
            "Entry {} should have no xattrs after removal but has {:?}",
            entry.name(),
            entry.xattrs()
        );
    })
    .unwrap();
}
