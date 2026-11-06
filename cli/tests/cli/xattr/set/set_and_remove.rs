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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
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
        "--overwrite",
        "-f",
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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
        "xattr_multi/zstd.pna",
        "--remove",
        "user.a",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_multi/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.b", b"B")]
        )]
    );
}
