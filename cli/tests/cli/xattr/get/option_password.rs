use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A pre-generated encrypted archive exists.
/// Action: Run `pna xattr set` then `pna xattr get` with `--password`.
/// Expectation: The xattr value is correctly retrieved from the encrypted archive.
#[test]
fn xattr_get_with_password() {
    setup();
    // Use pre-generated encrypted archive (password: "password")
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_get_password/").unwrap();

    // Set an xattr first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_get_password/zstd_aes_ctr.pna",
        "--password",
        "password",
        "--name",
        "user.test",
        "--value",
        "test_value",
        "raw/empty.txt",
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
            "xattr_get_password/zstd_aes_ctr.pna",
            "raw/empty.txt",
            "--password",
            "password",
            "--dump",
        ])
        .assert();

    assert.stdout(concat!(
        "# file: raw/empty.txt\n",
        "user.test=\"test_value\"\n",
        "\n",
    ));
}

/// Precondition: A pre-generated encrypted archive with an xattr set on an entry.
/// Action: Run `pna xattr get` without providing a password.
/// Expectation: The xattr metadata is visible as it is stored outside the encrypted content.
#[test]
fn xattr_get_metadata_visible_without_password() {
    setup();
    // Use pre-generated encrypted archive (password: "password")
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_get_no_pass/").unwrap();

    // Set an xattr with correct password
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_get_no_pass/zstd_aes_ctr.pna",
        "--password",
        "password",
        "--name",
        "user.metadata",
        "--value",
        "visible",
        "raw/empty.txt",
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
            "xattr_get_no_pass/zstd_aes_ctr.pna",
            "raw/empty.txt",
            "--dump",
        ])
        .assert();

    // xattr metadata is stored outside encrypted content, so it's visible
    assert.stdout(concat!(
        "# file: raw/empty.txt\n",
        "user.metadata=\"visible\"\n",
        "\n",
    ));
}
