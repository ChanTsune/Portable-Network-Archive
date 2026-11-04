use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A multipart archive exists spanning multiple files.
/// Action: Run `pna xattr set` on the first part to set an extended attribute.
/// Expectation: The xattr is applied and the archive is consolidated into a single file.
#[test]
fn xattr_set_on_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "xattr_multipart/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        "xattr_multipart/archive.pna",
        "--overwrite",
        "xattr_multipart/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "-f",
        "xattr_multipart/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "xattr_multipart/split/",
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
        "xattr_multipart/split/archive.part1.pna",
        "--name",
        "user.multipart",
        "--value",
        "from_split",
        "xattr_multipart/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::xattrs_by_entry("xattr_multipart/split/archive.pna", None),
        vec![(
            "xattr_multipart/in/raw/empty.txt".to_string(),
            vec![archive::xattr("user.multipart", b"from_split")]
        )]
    );
}

/// Precondition: A multipart archive exists with multiple entries across parts.
/// Action: Run `pna xattr set` with glob pattern to set xattr on multiple entries.
/// Expectation: The xattr is applied to all matching entries from all parts.
#[test]
fn xattr_set_multiple_entries_multipart() {
    setup();
    TestResources::extract_in("raw/", "xattr_multipart_multi/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        "xattr_multipart_multi/archive.pna",
        "--overwrite",
        "xattr_multipart_multi/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "-f",
        "xattr_multipart_multi/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "xattr_multipart_multi/split/",
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
        "xattr_multipart_multi/split/archive.part1.pna",
        "--name",
        "user.filetype",
        "--value",
        "text",
        "**/*.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut applied_sorted_by_name =
        archive::xattrs_by_entry("xattr_multipart_multi/split/archive.pna", None);
    applied_sorted_by_name.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        applied_sorted_by_name,
        vec![
            (
                "xattr_multipart_multi/in/raw/empty.txt".to_string(),
                vec![archive::xattr("user.filetype", b"text")]
            ),
            (
                "xattr_multipart_multi/in/raw/first/second/third/pna.txt".to_string(),
                vec![archive::xattr("user.filetype", b"text")]
            ),
            (
                "xattr_multipart_multi/in/raw/parent/child.txt".to_string(),
                vec![archive::xattr("user.filetype", b"text")]
            ),
            (
                "xattr_multipart_multi/in/raw/text.txt".to_string(),
                vec![archive::xattr("user.filetype", b"text")]
            ),
        ]
    );
}
