//! Tests for migrating archives with ACL data from 0.19.1 format to latest format.
//!
//! The 0.19.1 format stored each ACL entry as a self-describing `faCe` chunk with
//! an embedded platform prefix. Migration re-encodes them as one `faCl` platform
//! chunk followed by platform-less `faCe` chunks.

use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use pna::Chunk;
use portable_network_archive::cli;

const POSIX_ACL_ENTRIES: &[&str] = &[":u::allow:r|w|x", ":g::allow:r|w", ":o::allow:r"];
const MACOS_ACL_ENTRIES: &[&str] = &[":g:everyone:allow:r|w|x|delete|append|chown"];
const WINDOWS_ACL_ENTRIES: &[&str] = &[concat!(
    ":g:everyone:allow:r|w|x|delete|append|delete_child|readattr|writeattr|",
    "readextattr|writeextattr|readsecurity|writesecurity|chown|sync|read_data|write_data"
)];

fn assert_migrated_acl(platform: &str, acl_entries: &[&str]) {
    let source = format!("0.19.1/{platform}_acl.pna");
    let migrated = format!("migrate_{platform}_acl/migrated.pna");
    TestResources::extract_in(&source, ".").unwrap();

    cli::Cli::try_parse_from([
        "pna", "--quiet", "migrate", "-f", &source, "--output", &migrated,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // `acl get` renders the 0.19.1 format identically, so only the chunk bytes
    // can tell a conversion from a pass-through.
    let facl = pna::ChunkType::private(*b"faCl").unwrap();
    let face = pna::ChunkType::private(*b"faCe").unwrap();
    let mut expected = vec![(facl, platform.as_bytes())];
    expected.extend(acl_entries.iter().map(|ace| (face, ace.as_bytes())));
    let mut entries = 0;
    archive::for_each_entry(&migrated, |entry| {
        let acl_chunks: Vec<_> = entry
            .extra_chunks()
            .iter()
            .filter(|c| c.ty() == facl || c.ty() == face)
            .map(|c| (c.ty(), c.data()))
            .collect();
        assert_eq!(acl_chunks, expected);
        entries += 1;
    })
    .unwrap();
    assert_eq!(entries, 1);
}

/// Precondition: A 0.19.1 format archive with Linux ACL data exists.
/// Action: Run `pna migrate` to convert to latest format.
/// Expectation: The migrated entry stores the original ACL entries as one faCl
/// platform chunk followed by platform-less faCe chunks.
#[test]
fn migrate_linux_acl() {
    setup();
    assert_migrated_acl("linux", POSIX_ACL_ENTRIES);
}

/// Precondition: A 0.19.1 format archive with macOS ACL data exists.
/// Action: Run `pna migrate` to convert to latest format.
/// Expectation: The migrated entry stores the original ACL entries as one faCl
/// platform chunk followed by platform-less faCe chunks.
#[test]
fn migrate_macos_acl() {
    setup();
    assert_migrated_acl("macos", MACOS_ACL_ENTRIES);
}

/// Precondition: A 0.19.1 format archive with FreeBSD ACL data exists.
/// Action: Run `pna migrate` to convert to latest format.
/// Expectation: The migrated entry stores the original ACL entries as one faCl
/// platform chunk followed by platform-less faCe chunks.
#[test]
fn migrate_freebsd_acl() {
    setup();
    assert_migrated_acl("freebsd", POSIX_ACL_ENTRIES);
}

/// Precondition: A 0.19.1 format archive with Windows ACL data exists.
/// Action: Run `pna migrate` to convert to latest format.
/// Expectation: The migrated entry stores the original ACL entries as one faCl
/// platform chunk followed by platform-less faCe chunks.
#[test]
fn migrate_windows_acl() {
    setup();
    assert_migrated_acl("windows", WINDOWS_ACL_ENTRIES);
}
