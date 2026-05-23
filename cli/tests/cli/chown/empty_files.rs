use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::BTreeMap;

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` without specifying any file arguments.
/// Expectation: The command returns immediately without modifying the archive.
#[test]
fn chown_empty_files_is_noop() {
    setup();

    archive::create_archive_with_permissions(
        "chown_empty_files.pna",
        &[
            FileEntryDef {
                path: "a.txt",
                content: b"aaa",
                permission: 0o644,
            },
            FileEntryDef {
                path: "b.txt",
                content: b"bbb",
                permission: 0o755,
            },
        ],
    )
    .unwrap();

    // Record metadata before chown
    let mut before = BTreeMap::new();
    archive::for_each_entry("chown_empty_files.pna", |entry| {
        before.insert(
            entry.header().path().to_string(),
            (
                entry.metadata().owner_uid().map(|v| v.get()),
                entry.metadata().owner_gid().map(|v| v.get()),
                entry
                    .metadata()
                    .owner_user_name()
                    .map(|v| v.as_str().to_string()),
                entry
                    .metadata()
                    .owner_group_name()
                    .map(|v| v.as_str().to_string()),
                entry.metadata().permission_mode().map(|v| v.get()),
            ),
        );
    })
    .unwrap();

    // Run chown with no file arguments
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_empty_files.pna",
        "new_user:new_group",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify all entries are unchanged
    let mut after_count = 0usize;
    archive::for_each_entry("chown_empty_files.pna", |entry| {
        after_count += 1;
        let path = entry.header().path().to_string();
        let original = before
            .get(&path)
            .unwrap_or_else(|| panic!("unexpected entry after chown: {path}"));
        let actual = (
            entry.metadata().owner_uid().map(|v| v.get()),
            entry.metadata().owner_gid().map(|v| v.get()),
            entry
                .metadata()
                .owner_user_name()
                .map(|v| v.as_str().to_string()),
            entry
                .metadata()
                .owner_group_name()
                .map(|v| v.as_str().to_string()),
            entry.metadata().permission_mode().map(|v| v.get()),
        );
        assert_eq!(&actual, original, "metadata should be unchanged for {path}");
    })
    .unwrap();
    assert_eq!(after_count, before.len(), "entry count should be preserved");
}
