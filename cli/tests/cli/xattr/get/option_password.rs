use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An encrypted archive has an xattr set on an entry.
/// Action: Run `pna xattr get` with `--password` to retrieve the xattr.
/// Expectation: The xattr value is correctly retrieved from the encrypted archive.
#[test]
fn xattr_get_with_password() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_password/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_password/archive.pna",
        "--overwrite",
        "xattr_get_password/in/",
        "--password",
        "get_password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set an xattr first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_get_password/archive.pna",
        "--password",
        "get_password",
        "--name",
        "user.test",
        "--value",
        "test_value",
        "xattr_get_password/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify using get command with password
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_password/archive.pna",
            "xattr_get_password/in/raw/empty.txt",
            "--password",
            "get_password",
            "--dump",
        ])
        .assert();

    assert.stdout(concat!(
        "# file: xattr_get_password/in/raw/empty.txt\n",
        "user.test=\"test_value\"\n",
        "\n",
    ));
}

/// Precondition: An encrypted archive has an xattr set on an entry.
/// Action: Run `pna xattr get` without providing a password.
/// Expectation: The xattr metadata is visible as it is stored outside the encrypted content.
#[test]
fn xattr_get_metadata_visible_without_password() {
    setup();
    TestResources::extract_in("raw/", "xattr_get_no_pass/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_get_no_pass/archive.pna",
        "--overwrite",
        "xattr_get_no_pass/in/",
        "--password",
        "correct_password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set an xattr with correct password
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_get_no_pass/archive.pna",
        "--password",
        "correct_password",
        "--name",
        "user.metadata",
        "--value",
        "visible",
        "xattr_get_no_pass/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Get without password - xattr metadata is visible (not encrypted)
    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "--quiet",
            "xattr",
            "get",
            "xattr_get_no_pass/archive.pna",
            "xattr_get_no_pass/in/raw/empty.txt",
            "--dump",
        ])
        .assert();

    // xattr metadata is stored outside encrypted content, so it's visible
    assert.stdout(concat!(
        "# file: xattr_get_no_pass/in/raw/empty.txt\n",
        "user.metadata=\"visible\"\n",
        "\n",
    ));
}
