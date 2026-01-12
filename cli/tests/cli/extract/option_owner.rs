//! Tests for owner-related options in the extract command.
//!
//! These tests verify that ownership-related flags work correctly:
//! - `--same-owner`: Restore original ownership from archive
//! - `--no-same-owner`: Extract as current user (skip ownership restoration)
//! - `--uname`: Override user name
//! - `--gname`: Override group name
//! - `--uid`: Override user ID
//! - `--gid`: Override group ID
//! - `--numeric-owner`: Use numeric IDs only, ignoring names

use crate::utils::setup;
#[cfg(unix)]
use crate::utils::unix::skip_if_not_root;
use clap::Parser;
use portable_network_archive::cli;
use std::fs::{self, File};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Definition for creating a file entry with specific owner information
struct OwnerEntryDef<'a> {
    path: &'a str,
    content: &'a [u8],
    uid: u64,
    uname: &'a str,
    gid: u64,
    gname: &'a str,
    permission: u16,
}

fn create_archive_with_owner(
    archive_path: impl AsRef<std::path::Path>,
    entries: &[OwnerEntryDef],
) -> std::io::Result<()> {
    let file = File::create(archive_path)?;
    let mut archive = pna::Archive::write_header(file)?;

    for entry_def in entries {
        let mut builder =
            pna::EntryBuilder::new_file(entry_def.path.into(), pna::WriteOptions::store())?;
        builder.permission(pna::Permission::new(
            entry_def.uid,
            entry_def.uname.into(),
            entry_def.gid,
            entry_def.gname.into(),
            entry_def.permission,
        ));
        builder.write_all(entry_def.content)?;
        let entry = builder.build()?;
        archive.add_entry(entry)?;
    }

    archive.finalize()?;
    Ok(())
}

/// Precondition: An archive contains files with specific uid/gid.
/// Action: Extract the archive with `--no-same-owner`.
/// Expectation: The extracted file is owned by the current user, not the archive's owner.
#[test]
#[cfg(unix)]
fn extract_with_no_same_owner_skips_ownership() {
    setup();

    let archive_uid = 1234;
    let archive_gid = 5678;

    // Create archive with specific owner info
    fs::create_dir_all("extract_no_same_owner").unwrap();
    create_archive_with_owner(
        "extract_no_same_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: archive_uid,
            uname: "testuser",
            gid: archive_gid,
            gname: "testgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --no-same-owner
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_no_same_owner/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_no_same_owner/out/",
        "--keep-permission",
        "--no-same-owner",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_no_same_owner/out/test.txt").unwrap();
    let current_uid = nix::unistd::Uid::effective().as_raw();
    let current_gid = nix::unistd::Gid::effective().as_raw();

    assert_eq!(
        meta.uid(),
        current_uid,
        "extracted file should be owned by current user, not archive's uid {}",
        archive_uid
    );
    assert_eq!(
        meta.gid(),
        current_gid,
        "extracted file should be owned by current group, not archive's gid {}",
        archive_gid
    );
}

/// Precondition: An archive contains files with specific uid/gid.
/// Action: Extract the archive with `--same-owner` as root.
/// Expectation: The extracted file has ownership matching the archive.
#[test]
#[cfg(unix)]
fn extract_with_same_owner_restores_ownership() {
    setup();
    skip_if_not_root!();

    let archive_uid = 65534; // nobody
    let archive_gid = 65534; // nogroup

    // Create archive with specific owner info
    fs::create_dir_all("extract_same_owner").unwrap();
    create_archive_with_owner(
        "extract_same_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: archive_uid,
            uname: "nobody",
            gid: archive_gid,
            gname: "nogroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --same-owner
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_same_owner/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_same_owner/out/",
        "--keep-permission",
        "--same-owner",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_same_owner/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        archive_uid as u32,
        "extracted file should have archive's uid"
    );
    assert_eq!(
        meta.gid(),
        archive_gid as u32,
        "extracted file should have archive's gid"
    );
}

/// Precondition: An archive contains files with uid/gid.
/// Action: Extract the archive with `--uid` override.
/// Expectation: The extracted file has the overridden uid.
#[test]
#[cfg(unix)]
fn extract_with_uid_override() {
    setup();
    skip_if_not_root!();

    let archive_uid = 1000;
    let override_uid = 65534;

    fs::create_dir_all("extract_uid_override").unwrap();
    create_archive_with_owner(
        "extract_uid_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: archive_uid,
            uname: "originaluser",
            gid: 1000,
            gname: "originalgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --uid override
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_uid_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uid_override/out/",
        "--keep-permission",
        "--same-owner",
        "--uid",
        &override_uid.to_string(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_uid_override/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        override_uid,
        "extracted file should have overridden uid"
    );
}

/// Precondition: An archive contains files with uid/gid.
/// Action: Extract the archive with `--gid` override.
/// Expectation: The extracted file has the overridden gid.
#[test]
#[cfg(unix)]
fn extract_with_gid_override() {
    setup();
    skip_if_not_root!();

    let archive_gid = 1000;
    let override_gid = 65534;

    fs::create_dir_all("extract_gid_override").unwrap();
    create_archive_with_owner(
        "extract_gid_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: 1000,
            uname: "originaluser",
            gid: archive_gid,
            gname: "originalgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --gid override
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_gid_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_gid_override/out/",
        "--keep-permission",
        "--same-owner",
        "--gid",
        &override_gid.to_string(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_gid_override/out/test.txt").unwrap();

    assert_eq!(
        meta.gid(),
        override_gid,
        "extracted file should have overridden gid"
    );
}

/// Precondition: An archive contains files with user/group names.
/// Action: Extract the archive with `--uname` override.
/// Expectation: The extracted file has ownership based on the overridden user name.
#[test]
#[cfg(unix)]
fn extract_with_uname_override() {
    setup();
    skip_if_not_root!();

    fs::create_dir_all("extract_uname_override").unwrap();
    create_archive_with_owner(
        "extract_uname_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: 1000,
            uname: "originaluser",
            gid: 1000,
            gname: "originalgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --uname override to "nobody"
    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_uname_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uname_override/out/",
        "--keep-permission",
        "--same-owner",
        "--uname",
        "nobody",
    ])
    .unwrap()
    .execute();

    // The extraction might fail if user lookup fails on some systems
    if let Err(e) = &result {
        let err_str = format!("{:#}", e);
        if err_str.contains("not found") || err_str.contains("No such") {
            eprintln!("Skipping test: user 'nobody' not found on this system");
            return;
        }
    }
    result.unwrap();

    let meta = fs::metadata("extract_uname_override/out/test.txt").unwrap();

    // Check that the uid corresponds to "nobody" (typically 65534 or 99)
    // We just verify that it's not the original uid
    assert_ne!(
        meta.uid(),
        1000,
        "extracted file should not have original uid after --uname override"
    );
}

/// Precondition: An archive contains files with user/group names.
/// Action: Extract the archive with `--gname` override.
/// Expectation: The extracted file has ownership based on the overridden group name.
#[test]
#[cfg(unix)]
fn extract_with_gname_override() {
    setup();
    skip_if_not_root!();

    fs::create_dir_all("extract_gname_override").unwrap();
    create_archive_with_owner(
        "extract_gname_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: 1000,
            uname: "originaluser",
            gid: 1000,
            gname: "originalgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --gname override to "nogroup" or "nobody"
    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_gname_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_gname_override/out/",
        "--keep-permission",
        "--same-owner",
        "--gname",
        "nogroup",
    ])
    .unwrap()
    .execute();

    // The extraction might fail if group lookup fails on some systems
    if let Err(e) = &result {
        let err_str = format!("{:#}", e);
        if err_str.contains("not found") || err_str.contains("No such") {
            eprintln!("Skipping test: group 'nogroup' not found on this system");
            return;
        }
    }
    result.unwrap();

    let meta = fs::metadata("extract_gname_override/out/test.txt").unwrap();

    // Check that the gid corresponds to "nogroup" (typically 65534 or 99)
    // We just verify that it's not the original gid
    assert_ne!(
        meta.gid(),
        1000,
        "extracted file should not have original gid after --gname override"
    );
}

/// Precondition: An archive contains files with user/group names.
/// Action: Extract the archive with `--numeric-owner`.
/// Expectation: User/group names are ignored, only numeric IDs are used.
#[test]
#[cfg(unix)]
fn extract_with_numeric_owner() {
    setup();
    skip_if_not_root!();

    let archive_uid = 65534;
    let archive_gid = 65534;

    fs::create_dir_all("extract_numeric_owner").unwrap();
    create_archive_with_owner(
        "extract_numeric_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: archive_uid,
            uname: "nonexistentuser12345", // This user name shouldn't exist
            gid: archive_gid,
            gname: "nonexistentgroup12345", // This group name shouldn't exist
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --numeric-owner (ignores names, uses only IDs)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_numeric_owner/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_numeric_owner/out/",
        "--keep-permission",
        "--same-owner",
        "--numeric-owner",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_numeric_owner/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        archive_uid as u32,
        "extracted file should use numeric uid from archive"
    );
    assert_eq!(
        meta.gid(),
        archive_gid as u32,
        "extracted file should use numeric gid from archive"
    );
}

/// Precondition: An archive contains files with uid/gid.
/// Action: Extract the archive with both `--uid` and `--gid` overrides.
/// Expectation: The extracted file has both overridden uid and gid.
#[test]
#[cfg(unix)]
fn extract_with_uid_and_gid_override() {
    setup();
    skip_if_not_root!();

    let override_uid = 65534;
    let override_gid = 65534;

    fs::create_dir_all("extract_uid_gid_override").unwrap();
    create_archive_with_owner(
        "extract_uid_gid_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: 1000,
            uname: "originaluser",
            gid: 1000,
            gname: "originalgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with both --uid and --gid overrides
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_uid_gid_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uid_gid_override/out/",
        "--keep-permission",
        "--same-owner",
        "--uid",
        &override_uid.to_string(),
        "--gid",
        &override_gid.to_string(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_uid_gid_override/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        override_uid,
        "extracted file should have overridden uid"
    );
    assert_eq!(
        meta.gid(),
        override_gid,
        "extracted file should have overridden gid"
    );
}

/// Precondition: An archive contains files with uid/gid and uname/gname.
/// Action: Extract the archive with both `--uid` and `--uname` specified.
/// Expectation: The `--uid` option takes precedence over `--uname`.
#[test]
#[cfg(unix)]
fn extract_with_uid_overrides_uname() {
    setup();
    skip_if_not_root!();

    let override_uid = 65534;

    fs::create_dir_all("extract_uid_overrides_uname").unwrap();
    create_archive_with_owner(
        "extract_uid_overrides_uname/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: 1000,
            uname: "originaluser",
            gid: 1000,
            gname: "originalgroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with both --uid and --uname (--uid should take precedence)
    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_uid_overrides_uname/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uid_overrides_uname/out/",
        "--keep-permission",
        "--same-owner",
        "--uid",
        &override_uid.to_string(),
        "--uname",
        "nobody",
    ])
    .unwrap()
    .execute();

    // Skip if user lookup fails
    if let Err(e) = &result {
        let err_str = format!("{:#}", e);
        if err_str.contains("not found") || err_str.contains("No such") {
            eprintln!("Skipping test: user lookup failed");
            return;
        }
    }
    result.unwrap();

    let meta = fs::metadata("extract_uid_overrides_uname/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        override_uid,
        "--uid should take precedence over --uname"
    );
}

/// Precondition: An archive contains files with specific uid/gid.
/// Action: Extract the archive with `--keep-permission` only (no explicit same-owner flag).
/// Expectation: When root, ownership is restored; when non-root, ownership is current user.
#[test]
#[cfg(unix)]
fn extract_default_owner_behavior() {
    setup();

    let archive_uid = 65534;
    let archive_gid = 65534;

    fs::create_dir_all("extract_default_owner").unwrap();
    create_archive_with_owner(
        "extract_default_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: archive_uid,
            uname: "nobody",
            gid: archive_gid,
            gname: "nogroup",
            permission: 0o644,
        }],
    )
    .unwrap();

    // Extract with --keep-permission only (no explicit --same-owner or --no-same-owner)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_default_owner/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_default_owner/out/",
        "--keep-permission",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_default_owner/out/test.txt").unwrap();
    let is_root = nix::unistd::Uid::effective().is_root();

    if is_root {
        // When root, default behavior restores ownership from archive
        assert_eq!(
            meta.uid(),
            archive_uid as u32,
            "root should restore archive's uid by default"
        );
        assert_eq!(
            meta.gid(),
            archive_gid as u32,
            "root should restore archive's gid by default"
        );
    } else {
        // When non-root, file is owned by current user
        let current_uid = nix::unistd::Uid::effective().as_raw();
        let current_gid = nix::unistd::Gid::effective().as_raw();
        assert_eq!(
            meta.uid(),
            current_uid,
            "non-root should extract as current user"
        );
        assert_eq!(
            meta.gid(),
            current_gid,
            "non-root should extract as current group"
        );
    }
}
