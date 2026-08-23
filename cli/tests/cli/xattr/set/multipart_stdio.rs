use crate::utils::{EmbedExt, TestResources, archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: A multipart archive exists spanning multiple files.
/// Action: Run `pna xattr set` on the first part with stdout as the destination.
/// Expectation: The xattr is applied and stdout contains one consolidated archive.
#[test]
fn xattr_set_on_multipart_archive() {
    setup();
    TestResources::extract_in("raw/", "xattr_multipart/in/").unwrap();

    // Create a regular archive first
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

    // Split the archive into multiple parts
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

    // Omitted output consolidates every part into one archive stream on stdout.
    let output = cargo_bin_cmd!("pna")
        .args([
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
        .output()
        .unwrap();
    assert!(output.status.success());
    fs::write("xattr_multipart/split/archive.pna", output.stdout).unwrap();

    // Verify xattr was applied in the consolidated output (archive.pna, not archive.part1.pna)
    archive::for_each_entry("xattr_multipart/split/archive.pna", |entry| {
        if entry.name() == "xattr_multipart/in/raw/empty.txt" {
            let xattrs = entry.metadata().xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.multipart");
            assert_eq!(xattrs[0].value(), b"from_split");
        }
    })
    .unwrap();
}
