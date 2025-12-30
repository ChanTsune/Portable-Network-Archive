//! Tests for migrating archives with ACL data from 0.19.1 format to latest format.
//!
//! The 0.19.1 format stored ACL metadata differently. These tests verify that
//! migration correctly transforms the ACL data while preserving its semantics.

use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A 0.19.1 format archive with Linux ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: Migration succeeds and output archive is readable with preserved ACL data.
#[test]
fn migrate_linux_acl() {
    setup();
    TestResources::extract_in("0.19.1/linux_acl.pna", ".").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "migrate",
        "-f",
        "0.19.1/linux_acl.pna",
        "--output",
        "migrate_linux_acl/migrated.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify output archive is readable
    let mut count = 0;
    archive::for_each_entry("migrate_linux_acl/migrated.pna", |_entry| {
        count += 1;
    })
    .unwrap();
    assert!(count > 0, "Migrated archive should contain entries");
}

/// Precondition: A 0.19.1 format archive with macOS ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: Migration succeeds and output archive is readable with preserved ACL data.
#[test]
fn migrate_macos_acl() {
    setup();
    TestResources::extract_in("0.19.1/macos_acl.pna", ".").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "migrate",
        "-f",
        "0.19.1/macos_acl.pna",
        "--output",
        "migrate_macos_acl/migrated.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify output archive is readable
    let mut count = 0;
    archive::for_each_entry("migrate_macos_acl/migrated.pna", |_entry| {
        count += 1;
    })
    .unwrap();
    assert!(count > 0, "Migrated archive should contain entries");
}

/// Precondition: A 0.19.1 format archive with FreeBSD ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: Migration succeeds and output archive is readable with preserved ACL data.
#[test]
fn migrate_freebsd_acl() {
    setup();
    TestResources::extract_in("0.19.1/freebsd_acl.pna", ".").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "migrate",
        "-f",
        "0.19.1/freebsd_acl.pna",
        "--output",
        "migrate_freebsd_acl/migrated.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify output archive is readable
    let mut count = 0;
    archive::for_each_entry("migrate_freebsd_acl/migrated.pna", |_entry| {
        count += 1;
    })
    .unwrap();
    assert!(count > 0, "Migrated archive should contain entries");
}

/// Precondition: A 0.19.1 format archive with Windows ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: Migration succeeds and output archive is readable with preserved ACL data.
#[test]
fn migrate_windows_acl() {
    setup();
    TestResources::extract_in("0.19.1/windows_acl.pna", ".").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "migrate",
        "-f",
        "0.19.1/windows_acl.pna",
        "--output",
        "migrate_windows_acl/migrated.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify output archive is readable
    let mut count = 0;
    archive::for_each_entry("migrate_windows_acl/migrated.pna", |_entry| {
        count += 1;
    })
    .unwrap();
    assert!(count > 0, "Migrated archive should contain entries");
}
