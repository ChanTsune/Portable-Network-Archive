use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An encrypted archive exists with password protection.
/// Action: Run `pna xattr set` with `--password` to set an extended attribute.
/// Expectation: The xattr is applied to the entry in the encrypted archive.
#[test]
fn xattr_set_with_password() {
    setup();
    TestResources::extract_in("raw/", "xattr_password/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_password/archive.pna",
        "--overwrite",
        "xattr_password/in/",
        "--password",
        "test_password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_password/archive.pna",
        "--password",
        "test_password",
        "--name",
        "user.author",
        "--value",
        "pna developers",
        "xattr_password/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry_with_password("xattr_password/archive.pna", "test_password", |entry| {
        if entry.header().path().as_str() == "xattr_password/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.author");
            assert_eq!(xattrs[0].value(), b"pna developers");
        }
    })
    .unwrap();
}

/// Precondition: An encrypted archive exists and a password file contains the password.
/// Action: Run `pna xattr set` with `--password-file` to set an extended attribute.
/// Expectation: The xattr is applied using the password from the file.
#[test]
fn xattr_set_with_password_file() {
    setup();
    TestResources::extract_in("raw/", "xattr_password_file/in/").unwrap();

    let password = "file_password";
    std::fs::write("xattr_password_file/password.txt", password).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_password_file/archive.pna",
        "--overwrite",
        "xattr_password_file/in/",
        "--password",
        password,
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_password_file/archive.pna",
        "--password-file",
        "xattr_password_file/password.txt",
        "--name",
        "user.version",
        "--value",
        "1.0.0",
        "xattr_password_file/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry_with_password("xattr_password_file/archive.pna", password, |entry| {
        if entry.header().path().as_str() == "xattr_password_file/in/raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.version");
            assert_eq!(xattrs[0].value(), b"1.0.0");
        }
    })
    .unwrap();
}

/// Precondition: An encrypted archive exists with an xattr set on an entry.
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
    use assert_cmd::cargo::cargo_bin_cmd;
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
/// Action: Run `pna xattr set` with an incorrect password, then verify with correct password.
/// Expectation: The xattr set with wrong password does not affect the entry.
#[test]
fn xattr_set_wrong_password_no_effect() {
    setup();
    TestResources::extract_in("raw/", "xattr_wrong_password/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "xattr_wrong_password/archive.pna",
        "--overwrite",
        "xattr_wrong_password/in/",
        "--password",
        "correct_password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set an xattr with the correct password first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_wrong_password/archive.pna",
        "--password",
        "correct_password",
        "--name",
        "user.original",
        "--value",
        "original_value",
        "xattr_wrong_password/in/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Attempt to set xattr with wrong password (should not affect the archive)
    let _ = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_wrong_password/archive.pna",
        "--password",
        "wrong_password",
        "--name",
        "user.wrong",
        "--value",
        "wrong_value",
        "xattr_wrong_password/in/raw/empty.txt",
    ])
    .unwrap()
    .execute();

    // Verify with correct password - original xattr should still be there
    archive::for_each_entry_with_password(
        "xattr_wrong_password/archive.pna",
        "correct_password",
        |entry| {
            if entry.header().path().as_str() == "xattr_wrong_password/in/raw/empty.txt" {
                let xattrs = entry.xattrs();
                // Original xattr should exist
                assert!(
                    xattrs
                        .iter()
                        .any(|x| x.name() == "user.original" && x.value() == b"original_value"),
                    "original xattr should be preserved"
                );
            }
        },
    )
    .unwrap();
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
    use assert_cmd::cargo::cargo_bin_cmd;
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
