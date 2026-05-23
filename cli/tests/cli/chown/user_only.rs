use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
#[allow(deprecated)]
use pna::Permission;
use pna::{Archive, EntryBuilder, EntryName, WriteOptions};
use portable_network_archive::cli;
use std::fs::File;
use std::io::Write;

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` with `user` (no colon) to change only the user.
/// Expectation: The target entry has updated uname/uid; gname/gid and permission bits are preserved.
#[test]
fn chown_user_only() {
    setup();

    archive::create_archive_with_permissions(
        "chown_user_only.pna",
        &[
            FileEntryDef {
                path: "target.txt",
                content: b"target",
                permission: 0o644,
            },
            FileEntryDef {
                path: "other.txt",
                content: b"other",
                permission: 0o755,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_user_only.pna",
        "new_user",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    let mut count = 0usize;
    archive::for_each_entry("chown_user_only.pna", |entry| {
        count += 1;
        match entry.header().path().as_str() {
            "target.txt" => {
                found = true;
                assert_eq!(
                    entry.metadata().owner_user_name().unwrap().as_str(),
                    "new_user"
                );
                assert_eq!(entry.metadata().owner_uid().unwrap().get(), u64::MAX);
                assert_eq!(
                    entry.metadata().owner_group_name().unwrap().as_str(),
                    "group"
                );
                assert_eq!(entry.metadata().owner_gid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().permission_mode().unwrap().get(), 0o644);
            }
            "other.txt" => {
                assert_eq!(entry.metadata().owner_user_name().unwrap().as_str(), "user");
                assert_eq!(
                    entry.metadata().owner_group_name().unwrap().as_str(),
                    "group"
                );
                assert_eq!(entry.metadata().owner_uid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().owner_gid().unwrap().get(), 1000);
                assert_eq!(entry.metadata().permission_mode().unwrap().get(), 0o755);
            }
            other => panic!("unexpected entry: {other}"),
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
    assert_eq!(count, 2, "archive should contain exactly 2 entries");
}

/// Precondition: An archive entry carries legacy fPRM metadata.
/// Action: Run `pna experimental chown` to change the user.
/// Expectation: Ownership is emitted as owner facets, and stale fPRM is removed.
#[test]
#[allow(deprecated)]
fn chown_user_only_drops_legacy_fprm() {
    setup();
    let path = "chown_legacy_fprm.pna";
    {
        let mut archive = Archive::write_header(File::create(path).unwrap()).unwrap();
        let mut builder = EntryBuilder::new_file(
            EntryName::from_utf8_preserve_root("target.txt"),
            WriteOptions::store(),
        )
        .unwrap();
        builder.permission(Permission::new(
            1000,
            "user".to_string(),
            1000,
            "group".to_string(),
            0o644,
        ));
        builder.write_all(b"target").unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
        archive.finalize().unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path,
        "new_user",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(path, |entry| {
        let metadata = entry.metadata();
        assert!(
            metadata.permission().is_none(),
            "legacy fPRM must be removed after chown"
        );
        assert_eq!(metadata.owner_user_name().unwrap().as_str(), "new_user");
        assert_eq!(metadata.owner_uid().unwrap().get(), u64::MAX);
        assert_eq!(metadata.owner_group_name().unwrap().as_str(), "group");
        assert_eq!(metadata.owner_gid().unwrap().get(), 1000);
        assert_eq!(metadata.permission_mode().unwrap().get(), 0o644);
    })
    .unwrap();
}
