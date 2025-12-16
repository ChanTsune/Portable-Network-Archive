use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete all entries from the archive
///         by `pna experimental delete` with a wildcard pattern matching everything.
/// Expectation: The resulting archive contains no entries.
#[test]
fn delete_all_entries() {
    setup();
    TestResources::extract_in("raw/", "delete_all_entries/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_all_entries/delete_all_entries.pna",
        "--overwrite",
        "delete_all_entries/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_all_entries/delete_all_entries.pna",
        "**/*",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entry_count = 0;
    archive::for_each_entry("delete_all_entries/delete_all_entries.pna", |_entry| {
        entry_count += 1;
    })
    .unwrap();

    assert_eq!(
        entry_count, 0,
        "archive should be empty after deleting all entries"
    );
}
