use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A pre-generated encrypted archive exists.
/// Action: Run `pna xattr set` with `--password` to set an extended attribute.
/// Expectation: The xattr is applied to the entry in the encrypted archive.
#[test]
fn xattr_set_with_password() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_password/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_password/zstd_aes_ctr.pna", Some("password")),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.author", b"pna developers")]
        )]
    );
}

/// Precondition: A pre-generated encrypted archive exists and a password file contains the password.
/// Action: Run `pna xattr set` with `--password-file` to set an extended attribute.
/// Expectation: The xattr is applied using the password from the file.
#[test]
fn xattr_set_with_password_file() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_password_file/").unwrap();

    let password = "password";
    std::fs::write("xattr_password_file/password.txt", password).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_password_file/zstd_aes_ctr.pna", Some(password)),
        vec![(
            "raw/empty.txt".to_string(),
            vec![archive::xattr("user.version", b"1.0.0")]
        )]
    );
}

/// Precondition: A pre-generated encrypted archive exists.
/// Action: Run `pna xattr set` with correct password, then with incorrect password.
/// Expectation: Xattrs are plaintext metadata chunks, so the wrong-password command still
/// applies its xattr; only the encrypted file content requires the correct password. The
/// archive is CTR (unauthenticated), so the wrong-password command's own result is
/// unconstrained — only the postcondition is asserted.
#[test]
fn xattr_set_wrong_password_updates_metadata_only() {
    setup();
    TestResources::extract_in("zstd_aes_ctr.pna", "xattr_wrong_password/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
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

    let _ = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "xattr",
        "set",
        "-f",
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

    assert_eq!(
        archive::xattrs_by_entry("xattr_wrong_password/zstd_aes_ctr.pna", Some("password")),
        vec![(
            "raw/empty.txt".to_string(),
            vec![
                archive::xattr("user.original", b"original_value"),
                archive::xattr("user.wrong", b"wrong_value"),
            ]
        )]
    );
}
