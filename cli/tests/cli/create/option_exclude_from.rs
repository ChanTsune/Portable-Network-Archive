use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: A file contains exclusion patterns (`**/*.txt`) and a directory contains various files.
/// Action: Run `pna create` with `--exclude-from` pointing to the pattern file.
/// Expectation: Files matching the patterns in the file are excluded.
#[test]
fn create_with_exclude_from() {
    setup();
    TestResources::extract_in("raw/", "create_with_exclude_from/in/").unwrap();
    let file_path = "create_with_exclude_from/exclude_list";
    fs::write(file_path, "**/*.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_exclude_from/exclude_from.pna",
        "--overwrite",
        "create_with_exclude_from/in/",
        "--exclude-from",
        file_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_with_exclude_from/exclude_from.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Verify included entries (non-.txt files)
    let required_entries = [
        "create_with_exclude_from/in/raw/images/icon.bmp",
        "create_with_exclude_from/in/raw/images/icon.png",
        "create_with_exclude_from/in/raw/images/icon.svg",
        "create_with_exclude_from/in/raw/pna/empty.pna",
        "create_with_exclude_from/in/raw/pna/nest.pna",
    ];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }

    // Verify excluded entries (.txt files)
    let excluded_entries = [
        "create_with_exclude_from/in/raw/first/second/third/pna.txt",
        "create_with_exclude_from/in/raw/parent/child.txt",
        "create_with_exclude_from/in/raw/empty.txt",
        "create_with_exclude_from/in/raw/text.txt",
    ];
    for excluded in excluded_entries {
        assert!(
            !seen.contains(excluded),
            "excluded entry should not be present: {excluded}"
        );
    }

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
