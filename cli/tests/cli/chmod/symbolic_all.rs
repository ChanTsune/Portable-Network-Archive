use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o644 (rw-r--r--).
/// Action: Run `pna experimental chmod` with `+x` (no target, defaults to all) to add execute.
/// Expectation: The archive entry's permission becomes 0o755 (rwxr-xr-x).
#[test]
fn chmod_symbolic_all_implicit_add_execute() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_all_add_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_all_add_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_all_add_x/archive.pna",
        "--overwrite",
        "chmod_sym_all_add_x/in/",
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
        "chmod_sym_all_add_x/archive.pna",
        "--",
        "+x",
        "chmod_sym_all_add_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_all_add_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_all_add_x/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_a_add_x/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_a_add_x/in/raw/text.txt",
        fs::Permissions::from_mode(0o666),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_a_add_x/archive.pna",
        "--overwrite",
        "chmod_sym_a_add_x/in/",
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
        "chmod_sym_a_add_x/archive.pna",
        "a+x",
        "chmod_sym_a_add_x/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_a_add_x/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_a_add_x/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_a_rm_w/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_a_rm_w/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_a_rm_w/archive.pna",
        "--overwrite",
        "chmod_sym_a_rm_w/in/",
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
        "chmod_sym_a_rm_w/archive.pna",
        "a-w",
        "chmod_sym_a_rm_w/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_a_rm_w/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_a_rm_w/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_sym_all_set_rw/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_all_set_rw/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_all_set_rw/archive.pna",
        "--overwrite",
        "chmod_sym_all_set_rw/in/",
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
        "chmod_sym_all_set_rw/archive.pna",
        "--",
        "=rw",
        "chmod_sym_all_set_rw/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_all_set_rw/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_all_set_rw/in/raw/text.txt" {
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
