use crate::utils::{EmbedExt, TestResources, archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// A file entry from the 0.33.0 `zstd_keep_permission` fixture, which
/// carries only legacy `fPRM` (no owner facets): `uid=501
/// uname="kaihatsutarou" gid=20 gname="staff"`, mode `0o100644` unmasked
/// (`0o644` once read back through `PermissionMode`).
const LEGACY_FIXTURE_ENTRY: &str = "raw/text.txt";
const LEGACY_FIXTURE: &str = "0.33.0/zstd_keep_permission.pna";

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
/// Expectation: The user is updated; the group side, untouched by the
/// command, keeps the value filled from the entry's legacy fPRM.
#[test]
fn chown_user_only_on_legacy_fprm_entry_preserves_group() {
    setup();
    TestResources::extract_in(LEGACY_FIXTURE, "chown_legacy_fprm/").unwrap();
    let path = format!("chown_legacy_fprm/{LEGACY_FIXTURE}");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path.as_str(),
        "new_user",
        LEGACY_FIXTURE_ENTRY,
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry(&path, |entry| {
        if entry.header().path().as_str() == LEGACY_FIXTURE_ENTRY {
            found = true;
            let metadata = entry.metadata();
            assert_eq!(metadata.owner_user_name().unwrap().as_str(), "new_user");
            assert_eq!(metadata.owner_uid().unwrap().get(), u64::MAX);
            // The group side wasn't targeted, so it's rescued from the
            // fixture's legacy fPRM ("staff"/20) rather than left absent.
            assert_eq!(metadata.owner_group_name().unwrap().as_str(), "staff");
            assert_eq!(metadata.owner_gid().unwrap().get(), 20);
            assert_eq!(metadata.permission_mode().unwrap().get(), 0o644);
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}

/// Precondition: An archive entry carries legacy fPRM metadata and no owner
/// facets. Action: Run `pna experimental chown --numeric-owner` with a
/// numeric `uid:gid` spec (no names). Expectation: uid/gid take the
/// requested numeric values, and the owner *names* come back absent rather
/// than resurrected from the still-present legacy fPRM — the case the
/// all-or-nothing rescue rule exists for: a per-facet merge would instead
/// see `owner_user_name`/`owner_group_name` as merely "unset" and refill
/// them from `fPRM`'s `uname`/`gname`.
#[test]
fn chown_numeric_owner_drops_legacy_fprm_names() {
    setup();
    TestResources::extract_in(LEGACY_FIXTURE, "chown_numeric_owner_legacy_fprm/").unwrap();
    let path = format!("chown_numeric_owner_legacy_fprm/{LEGACY_FIXTURE}");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path.as_str(),
        "1000:2000",
        LEGACY_FIXTURE_ENTRY,
        "--numeric-owner",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry(&path, |entry| {
        if entry.header().path().as_str() == LEGACY_FIXTURE_ENTRY {
            found = true;
            let metadata = entry.metadata();
            assert_eq!(metadata.owner_uid().unwrap().get(), 1000);
            assert_eq!(metadata.owner_gid().unwrap().get(), 2000);
            assert!(
                metadata.owner_user_name().is_none(),
                "numeric-only chown must not resurrect the legacy fPRM uname"
            );
            assert!(
                metadata.owner_group_name().is_none(),
                "numeric-only chown must not resurrect the legacy fPRM gname"
            );
            assert_eq!(metadata.permission_mode().unwrap().get(), 0o644);
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}
