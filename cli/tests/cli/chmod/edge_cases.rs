use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with mode `000` to remove all permissions.
/// Expectation: The archive entry's permission becomes 0o000 (---------).
#[test]
fn chmod_edge_set_no_permissions() {
    setup();
    TestResources::extract_in("raw/", "chmod_edge_000/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_000/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_000/archive.pna",
        "--overwrite",
        "chmod_edge_000/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_000/archive.pna",
        "000",
        "chmod_edge_000/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_000/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_000/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_edge_777/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_777/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_777/archive.pna",
        "--overwrite",
        "chmod_edge_777/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_777/archive.pna",
        "777",
        "chmod_edge_777/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_777/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_777/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_edge_idem/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_idem/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_idem/archive.pna",
        "--overwrite",
        "chmod_edge_idem/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Apply the same permission that already exists
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_idem/archive.pna",
        "644",
        "chmod_edge_idem/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_idem/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_idem/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_edge_clear/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_clear/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_clear/archive.pna",
        "--overwrite",
        "chmod_edge_clear/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_clear/archive.pna",
        "a=",
        "chmod_edge_clear/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_clear/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_clear/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_edge_full/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_full/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_full/archive.pna",
        "--overwrite",
        "chmod_edge_full/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_full/archive.pna",
        "a=rwx",
        "chmod_edge_full/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_full/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_full/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_edge_idem_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_idem_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_idem_x/archive.pna",
        "--overwrite",
        "chmod_edge_idem_x/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Add +x when it already has x
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_idem_x/archive.pna",
        "--",
        "+x",
        "chmod_edge_idem_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_idem_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_idem_x/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_edge_idem_no_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_edge_idem_no_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_edge_idem_no_x/archive.pna",
        "--overwrite",
        "chmod_edge_idem_no_x/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Remove -x when there's no x to remove
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_edge_idem_no_x/archive.pna",
        "--",
        "-x",
        "chmod_edge_idem_no_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_edge_idem_no_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_edge_idem_no_x/in/raw/text.txt" {
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
