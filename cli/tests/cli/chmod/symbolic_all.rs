use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with `+x` (no target, defaults to all) to add execute.
/// Expectation: The archive entry's permission becomes 0o755 (rwxr-xr-x).
#[test]
fn chmod_symbolic_all_implicit_add_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_all_add_x.pna",
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
        "chmod_sym_all_add_x.pna",
        "--",
        "+x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_all_add_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o755,
                "+x on 0o644 should yield 0o755"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o666 (rw-rw-rw-).
/// Action: Run `pna experimental chmod` with `a+x` (explicit all) to add execute for all.
/// Expectation: The archive entry's permission becomes 0o777 (rwxrwxrwx).
#[test]
fn chmod_symbolic_all_explicit_add_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_a_add_x.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o666,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_a_add_x.pna",
        "a+x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_a_add_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o777,
                "a+x on 0o666 should yield 0o777"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `a-w` to remove write permission for all.
/// Expectation: The archive entry's permission becomes 0o555 (r-xr-xr-x).
#[test]
fn chmod_symbolic_all_remove_write() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_a_rm_w.pna",
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
        "chmod_sym_a_rm_w.pna",
        "a-w",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_a_rm_w.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o555,
                "a-w on 0o777 should yield 0o555"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Run `pna experimental chmod` with `=rw` to set read-write for all.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-).
#[test]
fn chmod_symbolic_all_set_readwrite() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_all_set_rw.pna",
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
        "chmod_sym_all_set_rw.pna",
        "--",
        "=rw",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_all_set_rw.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o666,
                "=rw on 0o000 should yield 0o666"
            );
        }
    })
    .unwrap();
}
