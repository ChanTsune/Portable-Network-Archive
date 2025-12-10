use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o000 (---------).
/// Action: Run `pna experimental chmod` with `u=rwx,g=rx,o=r` (common 754 pattern).
/// Expectation: The archive entry's permission becomes 0o754 (rwxr-xr--).
#[test]
fn chmod_multiple_clauses_standard_754() {
    setup();
    TestResources::extract_in("raw/", "chmod_multi_754/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_multi_754/in/raw/text.txt",
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_multi_754/archive.pna",
        "--overwrite",
        "chmod_multi_754/in/",
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
        "chmod_multi_754/archive.pna",
        "u=rwx,g=rx,o=r",
        "chmod_multi_754/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_754/archive.pna", |entry| {
        if entry.header().path() == "chmod_multi_754/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_multi_640/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_multi_640/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_multi_640/archive.pna",
        "--overwrite",
        "chmod_multi_640/in/",
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
        "chmod_multi_640/archive.pna",
        "u=rw,g=r,o=",
        "chmod_multi_640/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_640/archive.pna", |entry| {
        if entry.header().path() == "chmod_multi_640/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_multi_mixed/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_multi_mixed/in/raw/text.txt",
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_multi_mixed/archive.pna",
        "--overwrite",
        "chmod_multi_mixed/in/",
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
        "chmod_multi_mixed/archive.pna",
        "u+x,g+w,o-r",
        "chmod_multi_mixed/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_mixed/archive.pna", |entry| {
        if entry.header().path() == "chmod_multi_mixed/in/raw/text.txt" {
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
    TestResources::extract_in("raw/", "chmod_multi_combined/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_multi_combined/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_multi_combined/archive.pna",
        "--overwrite",
        "chmod_multi_combined/in/",
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
        "chmod_multi_combined/archive.pna",
        "ug=rwx,o=rx",
        "chmod_multi_combined/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_multi_combined/archive.pna", |entry| {
        if entry.header().path() == "chmod_multi_combined/in/raw/text.txt" {
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
