use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` with `user:group` to change both owner and group.
/// Expectation: The target entry has updated uname/gname/uid/gid; permission bits are preserved.
#[test]
fn chown_user_and_group() {
    setup();

    archive::create_archive_with_permissions(
        "chown_user_and_group.pna",
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
        "chown_user_and_group.pna",
        "new_user:new_group",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    let mut count = 0usize;
    archive::for_each_entry("chown_user_and_group.pna", |entry| {
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
                    "new_group"
                );
                assert_eq!(entry.metadata().owner_gid().unwrap().get(), u64::MAX);
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
