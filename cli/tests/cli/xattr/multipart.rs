use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A multipart archive exists spanning multiple files.
/// Action: Run `pna xattr set` on the first part to set an extended attribute.
/// Expectation: The xattr is applied and the archive is consolidated into a single file.
#[test]
fn xattr_set_on_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "xattr_multipart/in/").unwrap();

    // Create a regular archive first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "xattr_multipart/archive.pna",
        "--overwrite",
        "xattr_multipart/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Split the archive into multiple parts
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
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

    // Set xattr on an entry in the multipart archive (consolidates to archive.pna)
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

    // Verify xattr was applied in the consolidated output (archive.pna, not archive.part1.pna)
    archive::for_each_entry("xattr_multipart/split/archive.pna", |entry| {
        if entry.name() == "xattr_multipart/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.multipart");
            assert_eq!(xattrs[0].value(), b"from_split");
        }
    })
    .unwrap();
}

/// Precondition: A multipart archive exists with entries spread across parts.
/// Action: Run `pna xattr get` on the first part to retrieve entries from all parts.
/// Expectation: Entries from all parts are accessible via the first part.
#[test]
fn xattr_get_from_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "xattr_multipart_get/in/").unwrap();

    // Create archive and set xattr before splitting
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "xattr_multipart_get/archive.pna",
        "--overwrite",
        "xattr_multipart_get/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set xattr on the archive before splitting
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
        "xattr_multipart_get/archive.pna",
        "--name",
        "user.test",
        "--value",
        "test_value",
        "xattr_multipart_get/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Split the archive with xattr into multiple parts
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "xattr_multipart_get/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "xattr_multipart_get/split/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Get xattr from the multipart archive via the first part
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_multipart_get/split/archive.part1.pna",
            "xattr_multipart_get/in/raw/empty.txt",
            "--dump",
        ])
        .assert();

    assert.stdout(concat!(
        "# file: xattr_multipart_get/in/raw/empty.txt\n",
        "user.test=\"test_value\"\n",
        "\n",
    ));
}

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

    // Set xattr on all .txt files using glob pattern (consolidates to archive.pna)
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

    // Verify xattr was applied to all matching entries in consolidated archive
    let mut txt_count = 0;
    archive::for_each_entry("xattr_multipart_multi/split/archive.pna", |entry| {
        let path = entry.name();
        if path.as_str().ends_with(".txt") {
            txt_count += 1;
            let xattrs = entry.xattrs();
            assert_eq!(
                xattrs.len(),
                1,
                "txt file {path} should have exactly one xattr"
            );
            assert_eq!(xattrs[0].name(), "user.filetype");
            assert_eq!(xattrs[0].value(), b"text");
        } else {
            assert!(
                entry.xattrs().is_empty(),
                "non-txt file {path} should have no xattrs"
            );
        }
    })
    .unwrap();

    assert!(txt_count > 0, "should have found at least one .txt file");
}
