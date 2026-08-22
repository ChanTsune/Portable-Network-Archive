use crate::utils::{EmbedExt, TestResources, archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

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

/// Precondition: An archive entry carries legacy fPRM metadata (no owner facets).
/// Action: Run `pna experimental chown` to change uid and gid.
/// Expectation: The requested uid/gid are set as owner facets, the permission mode
/// survives, and the stale fPRM chunk is removed.
#[test]
fn chown_updates_ownership_of_a_legacy_fprm_archive() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "chown_legacy_fprm/").unwrap();
    let archive = "chown_legacy_fprm/0.33.0/zstd_keep_all.pna";

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        archive,
        "1000:1000",
        "**",
        "--numeric-owner",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = 0usize;
    archive::for_each_entry(archive, |entry| {
        seen += 1;
        let m = entry.metadata();
        assert_eq!(m.owner_uid().map(|v| v.get()), Some(1000));
        assert_eq!(m.owner_gid().map(|v| v.get()), Some(1000));
        assert!(m.owner_user_name().is_none());
        assert!(m.owner_group_name().is_none());
        let expected_mode = if entry.header().data_kind() == pna::DataKind::DIRECTORY {
            0o755
        } else {
            0o644
        };
        assert_eq!(
            m.permission_mode().map(|v| v.get()),
            Some(expected_mode),
            "mode must survive on {}",
            entry.header().path()
        );
    })
    .unwrap();
    assert_eq!(seen, 16, "fixture has 16 entries");
    let bytes = std::fs::read(archive).unwrap();
    assert!(
        !bytes.windows(4).any(|w| w == b"fPRM"),
        "legacy fPRM must be removed after chown"
    );
}
