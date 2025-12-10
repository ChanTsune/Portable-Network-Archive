use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Run `pna experimental chmod` with `u=rwx,g=rx,o=r` (common 754 pattern).
/// Expectation: The archive entry's permission becomes 0o754 (rwxr-xr--).
#[test]
fn chmod_multiple_clauses_standard_754() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_multi_754.pna",
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
        "chmod_multi_754.pna",
        "u=rwx,g=rx,o=r",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_754.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o754,
                "u=rwx,g=rx,o=r on 0o000 should yield 0o754"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `u=rw,g=r,o=` (644-like but with o=).
/// Expectation: The archive entry's permission becomes 0o640 (rw-r-----).
#[test]
fn chmod_multiple_clauses_restrictive() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_multi_640.pna",
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
        "chmod_multi_640.pna",
        "u=rw,g=r,o=",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_640.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o640,
                "u=rw,g=r,o= on 0o777 should yield 0o640"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with `u+x,g+w,o-r` (mixed add/remove).
/// Expectation: The archive entry's permission becomes 0o760 (rwxrw----).
#[test]
fn chmod_multiple_clauses_mixed_operations() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_multi_mixed.pna",
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
        "chmod_multi_mixed.pna",
        "u+x,g+w,o-r",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_mixed.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o760,
                "u+x,g+w,o-r on 0o644 should yield 0o760"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `ug=rwx,o=rx` (combined targets with clauses).
/// Expectation: The archive entry's permission becomes 0o775 (rwxrwxr-x).
#[test]
fn chmod_multiple_clauses_combined_targets() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_multi_combined.pna",
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
        "chmod_multi_combined.pna",
        "ug=rwx,o=rx",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_combined.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o775,
                "ug=rwx,o=rx on 0o777 should yield 0o775"
            );
        }
    })
    .unwrap();
}
