use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o640 (rw-r-----).
/// Action: Run `pna experimental chmod` with `o+r` to add read permission for others.
/// Expectation: The archive entry's permission becomes 0o644 (rw-r--r--).
#[test]
fn chmod_symbolic_other_add_read() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_o_add_r.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o640,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_o_add_r.pna",
        "o+r",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_o_add_r.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o644,
                "o+r on 0o640 should yield 0o644"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `o-rwx` to remove all permissions for others.
/// Expectation: The archive entry's permission becomes 0o770 (rwxrwx---).
#[test]
fn chmod_symbolic_other_remove_all() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_o_rm_rwx.pna",
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
        "chmod_sym_o_rm_rwx.pna",
        "o-rwx",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_o_rm_rwx.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o770,
                "o-rwx on 0o777 should yield 0o770"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `o=` to clear all other permissions.
/// Expectation: The archive entry's permission becomes 0o770 (rwxrwx---).
#[test]
fn chmod_symbolic_other_set_none() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_o_set_none.pna",
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
        "chmod_sym_o_set_none.pna",
        "o=",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_o_set_none.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o770,
                "o= on 0o777 should yield 0o770"
            );
        }
    })
    .unwrap();
}
