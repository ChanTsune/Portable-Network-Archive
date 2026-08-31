use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A multipart archive exists with multiple entries across parts.
/// Action: Run `pna xattr set` with glob pattern to set xattr on multiple entries.
/// Expectation: The xattr is applied to all matching entries from all parts.
#[test]
fn xattr_set_multiple_entries_multipart() {
    setup();
    TestResources::extract_in("raw/", "xattr_multipart_multi/in/").unwrap();

    // Create and split archive
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

    // Set xattr on all .txt files using an explicit consolidated output.
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "--overwrite",
        "-f",
        "xattr_multipart_multi/split/archive.part1.pna",
        "--output",
        "xattr_multipart_multi/split/archive.pna",
        "--name",
        "user.filetype",
        "--value",
        "text",
        "**/*.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify xattr was applied to all matching entries in consolidated archive
    let mut txt_count = 0;
    archive::for_each_entry("xattr_multipart_multi/split/archive.pna", |entry| {
        let path = entry.name();
        if path.as_str().ends_with(".txt") {
            txt_count += 1;
            let xattrs = entry.metadata().xattrs();
            assert_eq!(
                xattrs.len(),
                1,
                "txt file {path} should have exactly one xattr"
            );
            assert_eq!(xattrs[0].name(), "user.filetype");
            assert_eq!(xattrs[0].value(), b"text");
        } else {
            assert!(
                entry.metadata().xattrs().is_empty(),
                "non-txt file {path} should have no xattrs"
            );
        }
    })
    .unwrap();

    assert!(txt_count > 0, "should have found at least one .txt file");
}
