use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o640 (rw-r-----).
/// Action: Run `pna experimental chmod` with `o+r` to add read permission for others.
/// Expectation: The archive entry's permission becomes 0o644 (rw-r--r--).
#[test]
fn chmod_symbolic_other_add_read() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_o_add_r/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_o_add_r/in/raw/text.txt",
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_o_add_r/archive.pna",
        "--overwrite",
        "chmod_sym_o_add_r/in/",
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
        "chmod_sym_o_add_r/archive.pna",
        "o+r",
        "chmod_sym_o_add_r/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_o_add_r/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_o_add_r/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_o_rm_rwx/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_o_rm_rwx/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_o_rm_rwx/archive.pna",
        "--overwrite",
        "chmod_sym_o_rm_rwx/in/",
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
        "chmod_sym_o_rm_rwx/archive.pna",
        "o-rwx",
        "chmod_sym_o_rm_rwx/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_o_rm_rwx/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_o_rm_rwx/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_o_set_none/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_o_set_none/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_o_set_none/archive.pna",
        "--overwrite",
        "chmod_sym_o_set_none/in/",
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
        "chmod_sym_o_set_none/archive.pna",
        "o=",
        "chmod_sym_o_set_none/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_o_set_none/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_o_set_none/in/raw/text.txt" {
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
