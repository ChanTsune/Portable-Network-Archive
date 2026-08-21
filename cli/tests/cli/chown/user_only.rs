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

/// Precondition: An archive written before `0.34.0` records ownership as fPRM.
/// Action: Run `pna experimental chown` to change the user of every entry.
/// Expectation: The new user is emitted as owner facets, and the group rescued
/// from fPRM is preserved.
#[test]
fn chown_user_only_rewrites_legacy_fprm_archive() {
    setup();
    TestResources::extract_in("0.33.0/zstd_keep_all.pna", "chown_legacy_fprm/").unwrap();
    let path = "chown_legacy_fprm/0.33.0/zstd_keep_all.pna";

    let mut pre = std::collections::BTreeMap::new();
    archive::for_each_entry(path, |entry| {
        let meta = entry.metadata();
        pre.insert(
            entry.header().path().to_string(),
            (
                meta.owner_gid().map(|v| v.get()),
                meta.owner_group_name().map(|v| v.as_str().to_owned()),
                meta.permission_mode().map(|v| v.get()),
            ),
        );
    })
    .unwrap();
    assert!(!pre.is_empty(), "fixture should contain entries");

    let mut args = vec![
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        path,
        "new_user",
    ];
    args.extend(pre.keys().map(String::as_str));
    args.push("--no-owner-lookup");
    cli::Cli::try_parse_from(args).unwrap().execute().unwrap();

    let mut count = 0usize;
    archive::for_each_entry(path, |entry| {
        count += 1;
        let entry_path = entry.header().path().to_string();
        let meta = entry.metadata();
        let expected = pre
            .get(&entry_path)
            .unwrap_or_else(|| panic!("unexpected entry after chown: {entry_path}"));
        assert_eq!(
            meta.owner_user_name().map(|v| v.as_str()),
            Some("new_user"),
            "uname {entry_path}"
        );
        assert_eq!(
            (
                meta.owner_gid().map(|v| v.get()),
                meta.owner_group_name().map(|v| v.as_str().to_owned()),
                meta.permission_mode().map(|v| v.get()),
            ),
            *expected,
            "group and mode {entry_path}"
        );
    })
    .unwrap();
    assert_eq!(count, pre.len(), "chown should preserve all entries");
}
