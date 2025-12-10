use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o600 (rw-------).
/// Action: Run `pna experimental chmod` with `ug+x` to add execute for user and group.
/// Expectation: The archive entry's permission becomes 0o710 (rwx--x---).
#[test]
fn chmod_symbolic_user_group_add_execute() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_ug_add_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_ug_add_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_ug_add_x/archive.pna",
        "--overwrite",
        "chmod_sym_ug_add_x/in/",
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
        "chmod_sym_ug_add_x/archive.pna",
        "ug+x",
        "chmod_sym_ug_add_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_ug_add_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_ug_add_x/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_go_rm_w/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_go_rm_w/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_go_rm_w/archive.pna",
        "--overwrite",
        "chmod_sym_go_rm_w/in/",
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
        "chmod_sym_go_rm_w/archive.pna",
        "go-w",
        "chmod_sym_go_rm_w/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_go_rm_w/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_go_rm_w/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_uo_set_rx/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_uo_set_rx/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_uo_set_rx/archive.pna",
        "--overwrite",
        "chmod_sym_uo_set_rx/in/",
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
        "chmod_sym_uo_set_rx/archive.pna",
        "uo=rx",
        "chmod_sym_uo_set_rx/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_uo_set_rx/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_uo_set_rx/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_ugo_set_rwx/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_ugo_set_rwx/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_ugo_set_rwx/archive.pna",
        "--overwrite",
        "chmod_sym_ugo_set_rwx/in/",
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
        "chmod_sym_ugo_set_rwx/archive.pna",
        "ugo=rwx",
        "chmod_sym_ugo_set_rwx/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_ugo_set_rwx/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_ugo_set_rwx/in/raw/text.txt" {
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
