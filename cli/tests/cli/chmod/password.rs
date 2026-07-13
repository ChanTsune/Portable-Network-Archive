use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";
const PASSWORD: &str = "password";
const WRONG_PASSWORD: &str = "wrong-password";

/// Precondition: An encrypted archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with password and `-x` to remove execute.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-).
#[test]
fn chmod_with_password() {
    setup();

    archive::create_encrypted_archive_with_permissions(
        "chmod_password.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
        PASSWORD,
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_password.pna",
        "--password",
        PASSWORD,
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry_with_password("chmod_password.pna", Some(PASSWORD), |entry| {
        if entry.header().path() == ENTRY_PATH {
            found = true;
            let mode = entry
                .metadata()
                .permission_mode()
                .expect("entry should have permission mode metadata")
                .get();
            assert_eq!(mode & 0o777, 0o666, "-x on 0o777 should yield 0o666");
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}

/// Precondition: A solid encrypted archive exists.
/// Action: Run `pna experimental chmod` with an incorrect password.
/// Expectation: The command fails and the archive bytes remain unchanged.
#[test]
fn chmod_wrong_password_on_solid_archive_fails_without_modifying_archive() {
    setup();
    fs::create_dir_all("chmod_solid_wrong_password").unwrap();
    let archive_path = "chmod_solid_wrong_password/archive.pna";

    archive::create_encrypted_solid_archive_with_permissions(
        archive_path,
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
        PASSWORD,
    )
    .unwrap();
    let original = fs::read(archive_path).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        archive_path,
        "--password",
        WRONG_PASSWORD,
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "solid archive chmod should fail with a wrong password"
    );
    assert_eq!(
        fs::read(archive_path).unwrap(),
        original,
        "failed chmod must leave the original archive untouched"
    );
    assert_eq!(
        archive::entry_mode_with_password(archive_path, ENTRY_PATH, Some(PASSWORD)),
        0o777
    );
}

/// Precondition: A non-solid encrypted archive exists.
/// Action: Run `pna experimental chmod` with an incorrect password.
/// Expectation: The command succeeds because normal-entry metadata can be
/// rewritten without decrypting file contents; the original password still
/// decrypts the data.
#[test]
fn chmod_wrong_password_on_normal_encrypted_archive_updates_metadata_only() {
    setup();
    fs::create_dir_all("chmod_normal_wrong_password").unwrap();
    let archive_path = "chmod_normal_wrong_password/archive.pna";

    archive::create_encrypted_archive_with_permissions(
        archive_path,
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
        PASSWORD,
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        archive_path,
        "--password",
        WRONG_PASSWORD,
        "600",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::entry_mode_with_password(archive_path, ENTRY_PATH, Some(PASSWORD)),
        0o600
    );
    assert_eq!(
        archive::entry_contents_with_password(archive_path, ENTRY_PATH, Some(PASSWORD)),
        ENTRY_CONTENT
    );
}
