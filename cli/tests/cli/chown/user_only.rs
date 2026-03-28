use crate::utils::{archive, archive::FileEntryDef, setup};
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
                let p = entry.metadata().permission().unwrap();
                assert_eq!(p.uname(), "new_user");
                assert_eq!(p.uid(), u64::MAX);
                assert_eq!(p.gname(), "group");
                assert_eq!(p.gid(), 1000);
                assert_eq!(p.permissions(), 0o644);
            }
            "other.txt" => {
                let p = entry.metadata().permission().unwrap();
                assert_eq!(p.uname(), "user");
                assert_eq!(p.gname(), "group");
                assert_eq!(p.uid(), 1000);
                assert_eq!(p.gid(), 1000);
                assert_eq!(p.permissions(), 0o755);
            }
            other => panic!("unexpected entry: {other}"),
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
    assert_eq!(count, 2, "archive should contain exactly 2 entries");
}
