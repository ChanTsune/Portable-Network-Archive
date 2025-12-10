use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";
const PASSWORD: &str = "password_file_test";

/// Precondition: An encrypted archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with password file and `-x` to remove execute.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-).
#[test]
fn chmod_with_password_file() {
    setup();

    let password_file_path = "chmod_password_file.txt";
    fs::write(password_file_path, PASSWORD).unwrap();

    archive::create_encrypted_archive_with_permissions(
        "chmod_password_file.pna",
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
        "chmod_password_file.pna",
        "--password-file",
        password_file_path,
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry_with_password("chmod_password_file.pna", Some(PASSWORD), |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o666,
                "-x on 0o777 should yield 0o666"
            );
        }
    })
    .unwrap();
}
