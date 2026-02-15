use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries by
///         `pna experimental delete` with `--files-from` and `--null`.
/// Expectation: Only the entries listed in the null-separated file are removed; all other files remain.
#[test]
fn delete_with_files_from_null() {
    setup();
    TestResources::extract_in("raw/", "delete_files_from_null/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_files_from_null/delete_files_from_null.pna",
        "--overwrite",
        "--no-keep-dir",
        "delete_files_from_null/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Write patterns separated by null characters instead of newlines
    let list_path = "delete_files_from_null/delete_list";
    fs::write(
        list_path,
        ["**/raw/empty.txt", "**/raw/text.txt"].join("\0"),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_files_from_null/delete_files_from_null.pna",
        "--files-from",
        list_path,
        "--null",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "delete_files_from_null/delete_files_from_null.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    // empty.txt and text.txt should be deleted
    assert!(
        !seen.contains("delete_files_from_null/in/raw/empty.txt"),
        "empty.txt should have been deleted"
    );
    assert!(
        !seen.contains("delete_files_from_null/in/raw/text.txt"),
        "text.txt should have been deleted"
    );

    // Other entries should remain
    for required in [
        "delete_files_from_null/in/raw/images/icon.svg",
        "delete_files_from_null/in/raw/images/icon.bmp",
        "delete_files_from_null/in/raw/pna/empty.pna",
        "delete_files_from_null/in/raw/pna/nest.pna",
        "delete_files_from_null/in/raw/images/icon.png",
        "delete_files_from_null/in/raw/parent/child.txt",
        "delete_files_from_null/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries by
///         `pna experimental delete` with `--files-from` and `--null` where the
///         list file contains patterns separated by newlines (not nulls).
/// Expectation: The entire content is treated as a single pattern (since --null expects
///              null separators), and the command errors because no match is found.
#[test]
fn delete_with_files_from_null_rejects_newline_separator() {
    setup();
    TestResources::extract_in("raw/", "delete_null_newline/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_null_newline/delete_null_newline.pna",
        "--overwrite",
        "--no-keep-dir",
        "delete_null_newline/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Write patterns separated by newlines (NOT null characters)
    // With --null, this should be treated as a single pattern containing a newline
    let list_path = "delete_null_newline/delete_list";
    fs::write(
        list_path,
        "**/raw/empty.txt\n**/raw/text.txt", // newline-separated, not null-separated
    )
    .unwrap();

    // This should fail because the pattern "**/raw/empty.txt\n**/raw/text.txt"
    // (treated as a single pattern due to --null) won't match any entry
    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_null_newline/delete_null_newline.pna",
        "--files-from",
        list_path,
        "--null",
        "--unstable",
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "should error when newline-separated patterns are used with --null"
    );
}
