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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
        "xattr_set_base64/zstd.pna",
        "--name",
        "user.base64",
        "--value",
        "0sSGVsbG8gV29ybGQ=",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_set_base64/zstd.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.base64", b"Hello World")]
        )]
    );
}
