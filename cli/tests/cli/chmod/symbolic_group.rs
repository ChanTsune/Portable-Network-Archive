use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o640 (rw-r-----).
/// Action: Run `pna experimental chmod` with `g+w` to add write permission for the group.
/// Expectation: The archive entry's permission becomes 0o660 (rw-rw----).
#[test]
fn chmod_symbolic_group_add_write() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_g_add_w/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_g_add_w/in/raw/text.txt",
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_g_add_w/archive.pna",
        "--overwrite",
        "chmod_sym_g_add_w/in/",
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
        "chmod_sym_g_add_w/archive.pna",
        "g+w",
        "chmod_sym_g_add_w/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_g_add_w/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_g_add_w/in/raw/text.txt" {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o660,
                "g+w on 0o640 should yield 0o660"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o775 (rwxrwxr-x).
/// Action: Run `pna experimental chmod` with `g-w` to remove write permission for the group.
/// Expectation: The archive entry's permission becomes 0o755 (rwxr-xr-x).
#[test]
fn chmod_symbolic_group_remove_write() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_g_rm_w/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_g_rm_w/in/raw/text.txt",
        fs::Permissions::from_mode(0o775),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_g_rm_w/archive.pna",
        "--overwrite",
        "chmod_sym_g_rm_w/in/",
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
        "chmod_sym_g_rm_w/archive.pna",
        "g-w",
        "chmod_sym_g_rm_w/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_g_rm_w/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_g_rm_w/in/raw/text.txt" {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o755,
                "g-w on 0o775 should yield 0o755"
            );
        }
    })
    .unwrap();
}

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `g=rx` to set group permission to read-execute only.
/// Expectation: The archive entry's permission becomes 0o757 (rwxr-xrwx).
#[test]
fn chmod_symbolic_group_set_readexec() {
    setup();
    TestResources::extract_in("raw/", "chmod_sym_g_set_rx/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_sym_g_set_rx/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_sym_g_set_rx/archive.pna",
        "--overwrite",
        "chmod_sym_g_set_rx/in/",
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
        "chmod_sym_g_set_rx/archive.pna",
        "g=rx",
        "chmod_sym_g_set_rx/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_sym_g_set_rx/archive.pna", |entry| {
        if entry.header().path() == "chmod_sym_g_set_rx/in/raw/text.txt" {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o757,
                "g=rx on 0o777 should yield 0o757"
            );
        }
    })
    .unwrap();
}
