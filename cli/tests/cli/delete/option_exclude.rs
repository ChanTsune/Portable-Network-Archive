use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries from
///         the archive using `pna experimental delete` with `--exclude`.
/// Expectation: Target entries except those excluded by `--exclude` are removed.
#[test]
fn delete_with_exclude() {
    setup();
    TestResources::extract_in("raw/", "delete_output_exclude/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_output_exclude/delete_output_exclude.pna",
        "--overwrite",
        "--no-keep-dir",
        "delete_output_exclude/in/",
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
        "delete_output_exclude/delete_output_exclude.pna",
        "**.pna",
        "--exclude",
        "**/empty.*",
        "--unstable",
        "--output",
        "delete_output_exclude/delete_excluded.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();

    archive::for_each_entry("delete_output_exclude/delete_excluded.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "delete_output_exclude/in/raw/parent/child.txt",
        "delete_output_exclude/in/raw/first/second/third/pna.txt",
        "delete_output_exclude/in/raw/pna/empty.pna", // --exclude **/empty.*
        "delete_output_exclude/in/raw/text.txt",
        "delete_output_exclude/in/raw/images/icon.svg",
        "delete_output_exclude/in/raw/empty.txt",
        "delete_output_exclude/in/raw/images/icon.png",
        "delete_output_exclude/in/raw/images/icon.bmp",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
