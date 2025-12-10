use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o600 (rw-------).
/// Action: Run `pna experimental chmod` with `ug+x` to add execute for user and group.
/// Expectation: The archive entry's permission becomes 0o710 (rwx--x---).
#[test]
fn chmod_symbolic_user_group_add_execute() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_ug_add_x.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o600,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_sym_ug_add_x.pna",
        "ug+x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_ug_add_x.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o710,
                "ug+x on 0o600 should yield 0o710"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `go-w` to remove write for group and other.
/// Expectation: The archive entry's permission becomes 0o755 (rwxr-xr-x).
#[test]
fn chmod_symbolic_group_other_remove_write() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_go_rm_w.pna",
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
        "chmod_sym_go_rm_w.pna",
        "go-w",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_go_rm_w.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o755,
                "go-w on 0o777 should yield 0o755"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `uo=rx` to set read-execute for user and other.
/// Expectation: The archive entry's permission becomes 0o575 (r-xrwxr-x).
#[test]
fn chmod_symbolic_user_other_set_readexec() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_uo_set_rx.pna",
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
        "chmod_sym_uo_set_rx.pna",
        "uo=rx",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_uo_set_rx.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o575,
                "uo=rx on 0o777 should yield 0o575"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Run `pna experimental chmod` with `ugo=rwx` to set full permissions for all targets.
/// Expectation: The archive entry's permission becomes 0o777 (rwxrwxrwx).
#[test]
fn chmod_symbolic_all_targets_set_full() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_sym_ugo_set_rwx.pna",
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
        "chmod_sym_ugo_set_rwx.pna",
        "ugo=rwx",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_ugo_set_rwx.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o777,
                "ugo=rwx on 0o000 should yield 0o777"
            );
        }
    })
    .unwrap();
}
