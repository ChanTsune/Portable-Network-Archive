use crate::utils::{archive, setup};
use clap::Parser;
use pna::{Archive, EntryName, FileEntryBuilder, Metadata};
use portable_network_archive::cli;
use std::fs::File;
use std::io::Write;

fn create_archive_with_owner_sids(archive_path: &str) {
    let mut archive = Archive::write_header(File::create(archive_path).unwrap()).unwrap();
    let mut builder =
        FileEntryBuilder::new(EntryName::from_utf8_preserve_root("target.txt")).unwrap();
    builder.metadata(
        Metadata::new()
            .with_owner_uid(Some(pna::OwnerUid::from(1000)))
            .with_owner_gid(Some(pna::OwnerGid::from(1000)))
            .with_owner_user_name(Some(pna::OwnerUserName::new("user").unwrap()))
            .with_owner_group_name(Some(pna::OwnerGroupName::new("group").unwrap()))
            .with_owner_user_sid(Some(pna::OwnerUserSid::new("S-1-5-21-1111").unwrap()))
            .with_owner_group_sid(Some(pna::OwnerGroupSid::new("S-1-5-21-2222").unwrap()))
            .with_permission_mode(Some(pna::PermissionMode::from(0o644))),
    );
    builder.write_all(b"target").unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();
    archive.finalize().unwrap();
}

fn current_username() -> String {
    std::env::var("USERNAME").unwrap()
}

/// Precondition: An archive entry carries owner names and SID strings.
/// Action: Run `pna experimental chown` with a user name, resolved against the system.
/// Expectation: The target entry has the resolved user's name and SID string; the group side is preserved.
#[test]
fn chown_user_name_lookup_records_sid() {
    setup();
    create_archive_with_owner_sids("chown_win_user_lookup.pna");
    let username = current_username();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_win_user_lookup.pna",
        username.as_str(),
        "target.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry("chown_win_user_lookup.pna", |entry| {
        found = true;
        let m = entry.metadata();
        assert_eq!(m.owner_user_name().unwrap().as_str(), username);
        assert_eq!(m.owner_uid().unwrap().get(), u64::MAX);
        let user_sid = m.owner_user_sid().unwrap().as_str();
        assert!(user_sid.starts_with("S-1-5-"), "user sid: {user_sid}");
        assert_ne!(user_sid, "S-1-5-21-1111");
        assert_eq!(m.owner_group_name().unwrap().as_str(), "group");
        assert_eq!(m.owner_gid().unwrap().get(), 1000);
        assert_eq!(m.owner_group_sid().unwrap().as_str(), "S-1-5-21-2222");
        assert_eq!(m.permission_mode().unwrap().get(), 0o644);
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}

/// Precondition: An archive entry carries owner names and SID strings.
/// Action: Run `pna experimental chown` with `user:group` names, resolved against the system.
/// Expectation: The target entry has the resolved user and group names and SID strings.
#[test]
fn chown_user_and_group_name_lookup_records_sids() {
    setup();
    create_archive_with_owner_sids("chown_win_user_group_lookup.pna");
    let username = current_username();
    let owner = format!("{username}:Users");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_win_user_group_lookup.pna",
        owner.as_str(),
        "target.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry("chown_win_user_group_lookup.pna", |entry| {
        found = true;
        let m = entry.metadata();
        assert_eq!(m.owner_user_name().unwrap().as_str(), username);
        assert_eq!(m.owner_uid().unwrap().get(), u64::MAX);
        let user_sid = m.owner_user_sid().unwrap().as_str();
        assert!(user_sid.starts_with("S-1-5-"), "user sid: {user_sid}");
        assert_ne!(user_sid, "S-1-5-21-1111");
        assert_eq!(m.owner_group_name().unwrap().as_str(), "Users");
        assert_eq!(m.owner_gid().unwrap().get(), u64::MAX);
        assert_eq!(m.owner_group_sid().unwrap().as_str(), "S-1-5-32-545");
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}

/// Precondition: An archive entry carries owner names and SID strings.
/// Action: Run `pna experimental chown` with the `user:` form, resolved against the system.
/// Expectation: The target entry has the resolved user and the user's primary group, each with its SID string.
#[test]
fn chown_user_login_group_lookup_records_primary_group() {
    setup();
    create_archive_with_owner_sids("chown_win_login_group_lookup.pna");
    let username = current_username();
    let owner = format!("{username}:");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_win_login_group_lookup.pna",
        owner.as_str(),
        "target.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry("chown_win_login_group_lookup.pna", |entry| {
        found = true;
        let m = entry.metadata();
        assert_eq!(m.owner_user_name().unwrap().as_str(), username);
        assert_eq!(m.owner_uid().unwrap().get(), u64::MAX);
        let user_sid = m.owner_user_sid().unwrap().as_str();
        assert!(user_sid.starts_with("S-1-5-"), "user sid: {user_sid}");
        assert!(!m.owner_group_name().unwrap().as_str().is_empty());
        assert_eq!(m.owner_gid().unwrap().get(), u64::MAX);
        let group_sid = m.owner_group_sid().unwrap().as_str();
        assert!(group_sid.ends_with("-513"), "group sid: {group_sid}");
        assert_ne!(group_sid, user_sid);
        assert_ne!(group_sid, "S-1-5-21-2222");
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}
