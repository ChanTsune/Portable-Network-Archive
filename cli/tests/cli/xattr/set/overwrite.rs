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
        "xattr",
        "set",
        "-f",
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
        "xattr",
        "set",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_overwrite/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.name", b"second")]
        )]
    );
}
