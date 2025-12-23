use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashSet;

/// Precondition: A directory contains various file types (`.txt`, `.bmp`, `.png`, `.svg`, `.pna`).
/// Action: Run `pna create` with `--include "**/*.txt"`.
/// Expectation: Only `.txt` files are included; other file types are excluded.
#[test]
fn create_with_include() {
    setup();
    TestResources::extract_in("raw/", "create_with_include/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_include/include.pna",
        "--overwrite",
        "create_with_include/in/",
        "--include",
        "**/*.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_with_include/include.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // Verify included entries (.txt files)
    let required_entries = [
        "create_with_include/in/raw/empty.txt",
        "create_with_include/in/raw/text.txt",
        "create_with_include/in/raw/first/second/third/pna.txt",
        "create_with_include/in/raw/parent/child.txt",
    ];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }

    // Verify excluded entries (non-.txt files)
    let excluded_entries = [
        "create_with_include/in/raw/images/icon.bmp",
        "create_with_include/in/raw/images/icon.png",
        "create_with_include/in/raw/images/icon.svg",
        "create_with_include/in/raw/pna/empty.pna",
        "create_with_include/in/raw/pna/nest.pna",
    ];
    for excluded in excluded_entries {
        assert!(
            !seen.contains(excluded),
            "excluded entry should not be present: {excluded}"
        );
    }

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
