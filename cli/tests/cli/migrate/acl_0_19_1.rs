//! Tests for migrating archives with ACL data from 0.19.1 format to latest format.
//!
//! The 0.19.1 format stored each ACL entry as a self-describing `faCe` chunk with
//! an embedded platform prefix. Migration re-encodes them as one `faCl` platform
//! chunk followed by platform-less `faCe` chunks. `acl get` renders both formats
//! identically, so these tests pin the migrated ACL contents against a hardcoded
//! dump and separately assert the chunk-level re-encoding actually happened.

use crate::utils::{EmbedExt, TestResources, archive, setup};
#[cfg(not(target_family = "wasm"))]
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;

const POSIX_ACL_ENTRIES: &[&str] = &[":u::allow:r|w|x", ":g::allow:r|w", ":o::allow:r"];
const MACOS_ACL_ENTRIES: &[&str] = &[":g:everyone:allow:r|w|x|delete|append|chown"];
const WINDOWS_ACL_ENTRIES: &[&str] = &[concat!(
    ":g:everyone:allow:r|w|x|delete|append|delete_child|readattr|writeattr|",
    "readextattr|writeextattr|readsecurity|writesecurity|chown|sync|read_data|write_data"
)];

struct MigrateAclCase {
    archive: &'static str,
    migrated: &'static str,
    entry: &'static str,
    platform: &'static str,
    acl_entries: &'static [&'static str],
}

#[cfg(not(target_family = "wasm"))]
fn expected_acl_dump(case: &MigrateAclCase) -> String {
    format!(
        "# file: {}\n# owner: \n# group: \n# platform: {}\n{}\n\n",
        case.entry,
        case.platform,
        case.acl_entries.join("\n")
    )
}

fn assert_migrated_acl(case: &MigrateAclCase) {
    TestResources::extract_in(case.archive, ".").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "migrate",
        "-f",
        case.archive,
        "--output",
        case.migrated,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // The migrated archive must dump exactly the ACL recorded in the 0.19.1 fixture.
    // Subprocess spawning is unavailable on wasm; the chunk-level assertions below
    // still run there.
    #[cfg(not(target_family = "wasm"))]
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            case.migrated,
            case.entry,
        ])
        .assert()
        .success()
        .stdout(expected_acl_dump(case));

    // `acl get` reads the 0.19.1 format as well, so the dump alone cannot tell a
    // conversion from a pass-through. The re-encoded form is one `faCl` platform
    // chunk plus one `faCe` chunk per ACL entry.
    let facl = pna::ChunkType::private(*b"faCl").unwrap();
    let face = pna::ChunkType::private(*b"faCe").unwrap();
    let mut entries = 0;
    archive::for_each_entry(case.migrated, |entry| {
        let platform_chunks: Vec<_> = entry
            .extra_chunks()
            .iter()
            .filter(|c| c.ty() == facl)
            .collect();
        assert_eq!(
            platform_chunks.len(),
            1,
            "migrated entry should have exactly one faCl platform chunk"
        );
        assert_eq!(platform_chunks[0].data(), case.platform.as_bytes());
        let ace_count = entry
            .extra_chunks()
            .iter()
            .filter(|c| c.ty() == face)
            .count();
        assert_eq!(ace_count, case.acl_entries.len());
        entries += 1;
    })
    .unwrap();
    assert_eq!(entries, 1);
}

/// Precondition: A 0.19.1 format archive with Linux ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: The migrated archive dumps the original ACL entries and stores
/// them re-encoded as faCl/faCe chunks.
#[test]
fn migrate_linux_acl() {
    setup();
    assert_migrated_acl(&MigrateAclCase {
        archive: "0.19.1/linux_acl.pna",
        migrated: "migrate_linux_acl/migrated.pna",
        entry: "linux_acl.txt",
        platform: "linux",
        acl_entries: POSIX_ACL_ENTRIES,
    });
}

/// Precondition: A 0.19.1 format archive with macOS ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: The migrated archive dumps the original ACL entries and stores
/// them re-encoded as faCl/faCe chunks.
#[test]
fn migrate_macos_acl() {
    setup();
    assert_migrated_acl(&MigrateAclCase {
        archive: "0.19.1/macos_acl.pna",
        migrated: "migrate_macos_acl/migrated.pna",
        entry: "macos_acl.txt",
        platform: "macos",
        acl_entries: MACOS_ACL_ENTRIES,
    });
}

/// Precondition: A 0.19.1 format archive with FreeBSD ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: The migrated archive dumps the original ACL entries and stores
/// them re-encoded as faCl/faCe chunks.
#[test]
fn migrate_freebsd_acl() {
    setup();
    assert_migrated_acl(&MigrateAclCase {
        archive: "0.19.1/freebsd_acl.pna",
        migrated: "migrate_freebsd_acl/migrated.pna",
        entry: "freebsd_acl.txt",
        platform: "freebsd",
        acl_entries: POSIX_ACL_ENTRIES,
    });
}

/// Precondition: A 0.19.1 format archive with Windows ACL data exists.
/// Action: Run `pna experimental migrate` to convert to latest format.
/// Expectation: The migrated archive dumps the original ACL entries and stores
/// them re-encoded as faCl/faCe chunks.
#[test]
fn migrate_windows_acl() {
    setup();
    assert_migrated_acl(&MigrateAclCase {
        archive: "0.19.1/windows_acl.pna",
        migrated: "migrate_windows_acl/migrated.pna",
        entry: "windows_acl.txt",
        platform: "windows",
        acl_entries: WINDOWS_ACL_ENTRIES,
    });
}
