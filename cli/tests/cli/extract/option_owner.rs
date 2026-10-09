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

#[cfg(unix)]
struct TestOwner {
    uid: u32,
    uname: String,
    gid: u32,
    gname: String,
}

#[cfg(unix)]
fn owner_for_account(name: &str) -> TestOwner {
    let user = nix::unistd::User::from_name(name)
        .unwrap_or_else(|error| panic!("failed to look up user `{name}`: {error}"))
        .unwrap_or_else(|| panic!("required user `{name}` is missing from the account database"));
    let group = nix::unistd::Group::from_gid(user.gid)
        .unwrap_or_else(|error| panic!("failed to look up primary group for `{name}`: {error}"))
        .unwrap_or_else(|| {
            panic!("primary group for `{name}` is missing from the account database")
        });

    TestOwner {
        uid: user.uid.as_raw(),
        uname: user.name,
        gid: user.gid.as_raw(),
        gname: group.name,
    }
}

#[cfg(unix)]
fn root_test_owner() -> TestOwner {
    let owner = owner_for_account("root");
    assert_eq!(owner.uid, 0, "the root account must have uid 0");
    owner
}

#[cfg(unix)]
fn non_root_test_owner() -> TestOwner {
    let owner = owner_for_account("nobody");
    let root = root_test_owner();
    assert_ne!(owner.uid, root.uid, "the test account must not be root");
    assert_ne!(
        owner.gid, root.gid,
        "the test account's primary group must differ from root's"
    );
    assert_ne!(
        owner.gname, root.gname,
        "the test account's primary group name must differ from root's"
    );
    owner
}

fn create_archive_with_owner(
    archive_path: impl AsRef<std::path::Path>,
    entries: &[OwnerEntryDef],
) -> std::io::Result<()> {
    let file = File::create(archive_path)?;
    let mut archive = pna::Archive::write_header(file)?;

    for entry_def in entries {
        let mut builder = pna::FileEntryBuilder::new(entry_def.path.into())?;
        builder.metadata(
            pna::Metadata::new()
                .with_owner_uid(Some(pna::OwnerUid::from(entry_def.uid)))
                .with_owner_gid(Some(pna::OwnerGid::from(entry_def.gid)))
                .with_owner_user_name(Some(pna::OwnerUserName::new(entry_def.uname).unwrap()))
                .with_owner_group_name(Some(pna::OwnerGroupName::new(entry_def.gname).unwrap()))
                .with_permission_mode(Some(pna::PermissionMode::from(entry_def.permission))),
        );
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

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();

    fs::create_dir_all("extract_same_owner").unwrap();
    create_archive_with_owner(
        "extract_same_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(owner.uid),
            uname: &owner.uname,
            gid: u64::from(owner.gid),
            gname: &owner.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
        owner.uid,
        "extracted file should have archive's uid"
    );
    assert_eq!(
        meta.gid(),
        owner.gid,
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
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_uid_override").unwrap();
    create_archive_with_owner(
        "extract_uid_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(root.uid),
            uname: &root.uname,
            gid: u64::from(owner.gid),
            gname: &owner.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_uid_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uid_override/out/",
        "--keep-permission",
        "--same-owner",
        "--uid",
        &owner.uid.to_string(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_uid_override/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        owner.uid,
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
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_gid_override").unwrap();
    create_archive_with_owner(
        "extract_gid_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(owner.uid),
            uname: &owner.uname,
            gid: u64::from(root.gid),
            gname: &root.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_gid_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_gid_override/out/",
        "--keep-permission",
        "--same-owner",
        "--gid",
        &owner.gid.to_string(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_gid_override/out/test.txt").unwrap();

    assert_eq!(
        meta.gid(),
        owner.gid,
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
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_uname_override").unwrap();
    create_archive_with_owner(
        "extract_uname_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(root.uid),
            uname: &root.uname,
            gid: u64::from(owner.gid),
            gname: &owner.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_uname_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uname_override/out/",
        "--keep-permission",
        "--same-owner",
        "--uname",
        &owner.uname,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_uname_override/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        owner.uid,
        "extracted file should use the uid resolved from --uname"
    );
}

/// Precondition: An archive contains files with user/group names.
/// Action: Extract the archive with `--gname` override.
/// Expectation: The extracted file has ownership based on the overridden group name.
#[test]
#[cfg(unix)]
fn extract_with_gname_override() {
    setup();
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_gname_override").unwrap();
    create_archive_with_owner(
        "extract_gname_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(owner.uid),
            uname: &owner.uname,
            gid: u64::from(root.gid),
            gname: &root.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_gname_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_gname_override/out/",
        "--keep-permission",
        "--same-owner",
        "--gname",
        &owner.gname,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_gname_override/out/test.txt").unwrap();

    assert_eq!(
        meta.gid(),
        owner.gid,
        "extracted file should use the gid resolved from --gname"
    );
}

/// Precondition: An archive contains files with user/group names.
/// Action: Extract the archive with `--numeric-owner`.
/// Expectation: User/group names are ignored, only numeric IDs are used.
#[test]
#[cfg(unix)]
fn extract_with_numeric_owner() {
    setup();
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_numeric_owner").unwrap();
    create_archive_with_owner(
        "extract_numeric_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(owner.uid),
            uname: &root.uname,
            gid: u64::from(owner.gid),
            gname: &root.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
        owner.uid,
        "extracted file should use numeric uid from archive"
    );
    assert_eq!(
        meta.gid(),
        owner.gid,
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
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_uid_gid_override").unwrap();
    create_archive_with_owner(
        "extract_uid_gid_override/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(root.uid),
            uname: &root.uname,
            gid: u64::from(root.gid),
            gname: &root.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_uid_gid_override/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uid_gid_override/out/",
        "--keep-permission",
        "--same-owner",
        "--uid",
        &owner.uid.to_string(),
        "--gid",
        &owner.gid.to_string(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_uid_gid_override/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        owner.uid,
        "extracted file should have overridden uid"
    );
    assert_eq!(
        meta.gid(),
        owner.gid,
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
    skip_unless!("root", nix::unistd::Uid::effective().is_root());

    let owner = non_root_test_owner();
    let root = root_test_owner();

    fs::create_dir_all("extract_uid_overrides_uname").unwrap();
    create_archive_with_owner(
        "extract_uid_overrides_uname/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(root.uid),
            uname: &root.uname,
            gid: u64::from(owner.gid),
            gname: &owner.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "extract_uid_overrides_uname/archive.pna",
        "--overwrite",
        "--out-dir",
        "extract_uid_overrides_uname/out/",
        "--keep-permission",
        "--same-owner",
        "--uid",
        &owner.uid.to_string(),
        "--uname",
        &root.uname,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let meta = fs::metadata("extract_uid_overrides_uname/out/test.txt").unwrap();

    assert_eq!(
        meta.uid(),
        owner.uid,
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

    let owner = non_root_test_owner();

    fs::create_dir_all("extract_default_owner").unwrap();
    create_archive_with_owner(
        "extract_default_owner/archive.pna",
        &[OwnerEntryDef {
            path: "test.txt",
            content: b"test content",
            uid: u64::from(owner.uid),
            uname: &owner.uname,
            gid: u64::from(owner.gid),
            gname: &owner.gname,
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
        assert_eq!(
            meta.uid(),
            owner.uid,
            "root should restore archive's uid by default"
        );
        assert_eq!(
            meta.gid(),
            owner.gid,
            "root should restore archive's gid by default"
        );
    } else {
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
