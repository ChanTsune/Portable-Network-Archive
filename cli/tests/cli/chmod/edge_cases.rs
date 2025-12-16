use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with mode `000` to remove all permissions.
/// Expectation: The archive entry's permission becomes 0o000 (---------).
#[test]
fn chmod_edge_set_no_permissions() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_000.pna",
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
        "chmod_edge_000.pna",
        "000",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_000.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o000,
                "000 on 0o777 should yield 0o000"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Run `pna experimental chmod` with mode `777` to grant all permissions.
/// Expectation: The archive entry's permission becomes 0o777 (rwxrwxrwx).
#[test]
fn chmod_edge_set_full_permissions() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_777.pna",
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
        "chmod_edge_777.pna",
        "777",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_777.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o777,
                "777 on 0o000 should yield 0o777"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with mode `644` (same as current).
/// Expectation: The operation succeeds and the permission remains 0o644 (idempotent).
#[test]
fn chmod_edge_idempotent_operation() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_idem.pna",
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
        "chmod_edge_idem.pna",
        "644",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_idem.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o644,
                "644 on 0o644 should remain 0o644"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o755 (rwxr-xr-x).
/// Action: Run `pna experimental chmod` with symbolic mode `a=` to clear all permissions.
/// Expectation: The archive entry's permission becomes 0o000 (---------).
#[test]
fn chmod_edge_symbolic_clear_all() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_clear.pna",
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
        "chmod_edge_clear.pna",
        "a=",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_clear.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o000,
                "a= on 0o755 should yield 0o000"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Run `pna experimental chmod` with symbolic mode `a=rwx` to set full permissions.
/// Expectation: The archive entry's permission becomes 0o777 (rwxrwxrwx).
#[test]
fn chmod_edge_symbolic_full_permissions() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_full.pna",
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
        "chmod_edge_full.pna",
        "a=rwx",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_full.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o777,
                "a=rwx on 0o000 should yield 0o777"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o755 (rwxr-xr-x).
/// Action: Run `pna experimental chmod` with symbolic mode `+x` on file that already has x.
/// Expectation: The operation succeeds and permission remains 0o755 (idempotent +x).
#[test]
fn chmod_edge_idempotent_add_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_idem_x.pna",
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
        "chmod_edge_idem_x.pna",
        "--",
        "+x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_idem_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o755,
                "+x on 0o755 should remain 0o755"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with symbolic mode `-x` on file without x.
/// Expectation: The operation succeeds and permission remains 0o644 (idempotent -x).
#[test]
fn chmod_edge_idempotent_remove_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_edge_idem_no_x.pna",
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
        "chmod_edge_idem_no_x.pna",
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_idem_no_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o644,
                "-x on 0o644 should remain 0o644"
            );
        }
    })
    .unwrap();
}
