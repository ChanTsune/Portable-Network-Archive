use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path};

const ENTRY_PATH: &str = "test.txt";

/// Precondition: A multipart archive exists.
/// Action: Run `pna experimental chmod` against the first part.
/// Expectation: The command reads all parts and writes the updated consolidated archive.
#[test]
fn chmod_multipart_archive_updates_consolidated_output() {
    setup();
    fs::create_dir_all("chmod_multipart/split").unwrap();
    let content = vec![b'x'; 4096];

    archive::create_archive_with_permissions(
        "chmod_multipart/archive.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: &content,
            permission: 0o777,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "-f",
        "chmod_multipart/archive.pna",
        "--overwrite",
        "--max-size",
        "1kb",
        "--out-dir",
        "chmod_multipart/split",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(
        Path::new("chmod_multipart/split/archive.part2.pna").exists(),
        "test archive should be split into multiple parts"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_multipart/split/archive.part1.pna",
        "600",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::entry_mode("chmod_multipart/split/archive.pna", ENTRY_PATH),
        0o600
    );
}
