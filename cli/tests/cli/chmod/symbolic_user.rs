use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with `u+x` to add execute permission for the user.
/// Expectation: The archive entry's permission becomes 0o744 (rwxr--r--).
#[test]
fn chmod_symbolic_user_add_execute() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_u_add_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_u_add_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_u_add_x/archive.pna",
        "--overwrite",
        "chmod_sym_u_add_x/in/",
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
        "chmod_sym_u_add_x/archive.pna",
        "u+x",
        "chmod_sym_u_add_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_add_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_u_add_x/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_u_rm_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_u_rm_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_u_rm_x/archive.pna",
        "--overwrite",
        "chmod_sym_u_rm_x/in/",
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
        "chmod_sym_u_rm_x/archive.pna",
        "u-x",
        "chmod_sym_u_rm_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_rm_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_u_rm_x/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_u_set_rw/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_u_set_rw/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_u_set_rw/archive.pna",
        "--overwrite",
        "chmod_sym_u_set_rw/in/",
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
        "chmod_sym_u_set_rw/archive.pna",
        "u=rw",
        "chmod_sym_u_set_rw/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_set_rw/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_u_set_rw/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_u_set_rwx/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_u_set_rwx/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_u_set_rwx/archive.pna",
        "--overwrite",
        "chmod_sym_u_set_rwx/in/",
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
        "chmod_sym_u_set_rwx/archive.pna",
        "u=rwx",
        "chmod_sym_u_set_rwx/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_u_set_rwx/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_u_set_rwx/in/raw/text.txt" {
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
