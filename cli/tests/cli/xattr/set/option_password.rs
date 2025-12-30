use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A pre-generated encrypted archive exists.
/// Action: Run `pna xattr set` with `--password` to set an extended attribute.
/// Expectation: The xattr is applied to the entry in the encrypted archive.
#[test]
fn xattr_set_with_password() {
    setup();
    // Use pre-generated encrypted archive (password: "password")
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_password/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_password/zstd_aes_ctr.pna",
        "--password",
        "password",
        "--name",
        "user.author",
        "--value",
        "pna developers",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry_with_password("xattr_password/zstd_aes_ctr.pna", "password", |entry| {
        if entry.name() == "raw/empty.txt" {
            let xattrs = entry.xattrs();
            assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
            assert_eq!(xattrs[0].name(), "user.author");
            assert_eq!(xattrs[0].value(), b"pna developers");
        } else {
            // Non-target entries should remain unaffected (no xattrs)
            assert!(
                entry.xattrs().is_empty(),
                "Entry {} should have no xattrs but has {:?}",
                entry.name(),
                entry.xattrs()
            );
        }
    })
    .unwrap();
}

/// Precondition: A pre-generated encrypted archive exists and a password file contains the password.
/// Action: Run `pna xattr set` with `--password-file` to set an extended attribute.
/// Expectation: The xattr is applied using the password from the file.
#[test]
fn xattr_set_with_password_file() {
    setup();
    // Use pre-generated encrypted archive (password: "password")
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_password_file/").unwrap();

    let password = "password";
    std::fs::write("xattr_password_file/password.txt", password).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_password_file/zstd_aes_ctr.pna",
        "--password-file",
        "xattr_password_file/password.txt",
        "--name",
        "user.version",
        "--value",
        "1.0.0",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry_with_password(
        "xattr_password_file/zstd_aes_ctr.pna",
        password,
        |entry| {
            if entry.name() == "raw/empty.txt" {
                let xattrs = entry.xattrs();
                assert_eq!(xattrs.len(), 1, "entry should have exactly one xattr");
                assert_eq!(xattrs[0].name(), "user.version");
                assert_eq!(xattrs[0].value(), b"1.0.0");
            } else {
                // Non-target entries should remain unaffected (no xattrs)
                assert!(
                    entry.xattrs().is_empty(),
                    "Entry {} should have no xattrs but has {:?}",
                    entry.name(),
                    entry.xattrs()
                );
            }
        },
    )
    .unwrap();
}

/// Precondition: A pre-generated encrypted archive exists.
/// Action: Run `pna xattr set` with correct password, then with incorrect password.
/// Expectation: The xattr set with wrong password does not affect the entry.
#[test]
fn xattr_set_wrong_password_no_effect() {
    setup();
    // Use pre-generated encrypted archive (password: "password")
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_wrong_password/").unwrap();

    // Set an xattr with the correct password first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "xattr_wrong_password/zstd_aes_ctr.pna",
        "--password",
        "password",
        "--name",
        "user.original",
        "--value",
        "original_value",
        "raw/empty.txt",
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
        "xattr_wrong_password/zstd_aes_ctr.pna",
        "--password",
        "wrong_password",
        "--name",
        "user.wrong",
        "--value",
        "wrong_value",
        "raw/empty.txt",
    ])
    .unwrap()
    .execute();

    // Verify with correct password - original xattr should still be there
    archive::for_each_entry_with_password(
        "xattr_wrong_password/zstd_aes_ctr.pna",
        "password",
        |entry| {
            if entry.name() == "raw/empty.txt" {
                let xattrs = entry.xattrs();
                // Original xattr should exist
                assert!(
                    xattrs
                        .iter()
                        .any(|x| x.name() == "user.original" && x.value() == b"original_value"),
                    "original xattr should be preserved"
                );
            } else {
                // Non-target entries should remain unaffected (no xattrs)
                assert!(
                    entry.xattrs().is_empty(),
                    "Entry {} should have no xattrs but has {:?}",
                    entry.name(),
                    entry.xattrs()
                );
            }
        },
    )
    .unwrap();
}
