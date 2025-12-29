use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

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
