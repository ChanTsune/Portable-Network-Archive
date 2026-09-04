use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive with multiple entries exists.
/// Action: Set an extended attribute with `--output` to a new path.
/// Expectation: The output archive has the xattr; the original is untouched.
#[test]
fn xattr_set_output() {
    setup();
    TestResources::extract_in("zstd.pna", "xattr_set_output/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_set_output/zstd.pna",
        "--output",
        "xattr_set_output/out.pna",
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_set_output/zstd.pna", None),
        vec![]
    );
    assert_eq!(
        archive::xattrs_by_entry("xattr_set_output/out.pna", None),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.name", b"pna developers!")]
        )]
    );
}
