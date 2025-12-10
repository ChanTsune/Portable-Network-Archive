use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with `u+x` to add execute permission for the user.
/// Expectation: The archive entry's permission becomes 0o744 (rwxr--r--).
#[test]
fn chmod_symbolic_user_add_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_u_add_x.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_u_add_x.pna",
        "u+x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_add_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o744,
                "u+x on 0o644 should yield 0o744"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o755 (rwxr-xr-x).
/// Action: Run `pna experimental chmod` with `u-x` to remove execute permission for the user.
/// Expectation: The archive entry's permission becomes 0o655 (rw-r-xr-x).
#[test]
fn chmod_symbolic_user_remove_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_u_rm_x.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o755,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_u_rm_x.pna",
        "u-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_rm_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o655,
                "u-x on 0o755 should yield 0o655"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `u=rw` to set user permission to read-write only.
/// Expectation: The archive entry's permission becomes 0o677 (rw-rwxrwx).
#[test]
fn chmod_symbolic_user_set_readwrite() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_u_set_rw.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_u_set_rw.pna",
        "u=rw",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_set_rw.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o677,
                "u=rw on 0o777 should yield 0o677"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o000 (--------).
/// Action: Run `pna experimental chmod` with `u=rwx` to set full user permissions.
/// Expectation: The archive entry's permission becomes 0o700 (rwx------).
#[test]
fn chmod_symbolic_user_set_full() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_u_set_rwx.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o000,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_u_set_rwx.pna",
        "u=rwx",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_set_rwx.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o700,
                "u=rwx on 0o000 should yield 0o700"
            );
        }
    })
    .unwrap();
}
