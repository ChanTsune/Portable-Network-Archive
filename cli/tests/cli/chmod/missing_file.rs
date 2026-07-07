use crate::utils::{EmbedExt, TestResources, archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains files, but one target file does not exist in the archive.
/// Action: Run `pna experimental chmod` targeting both existing and non-existing files.
/// Expectation: The command fails with an error when a specified file is not found.
#[test]
fn fail_with_missing_file() {
    setup();
    TestResources::extract_in("raw/", "chmod_missing/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "chmod_missing/archive.pna",
        "--overwrite",
        "chmod_missing/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_missing/archive.pna",
        "644",
        "chmod_missing/in/raw/empty.txt",
        "chmod_missing/in/raw/not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
}

/// Precondition: An archive contains one matching target and one missing target.
/// Action: Run `pna experimental chmod` over both targets.
/// Expectation: The command fails and no partial metadata update is persisted.
#[test]
fn missing_file_does_not_persist_partial_update() {
    setup();
    fs::create_dir_all("chmod_missing_atomic").unwrap();
    let archive_path = "chmod_missing_atomic/archive.pna";

    archive::create_archive_with_permissions(
        archive_path,
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
    )
    .unwrap();
    let original = fs::read(archive_path).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        archive_path,
        "600",
        ENTRY_PATH,
        "missing.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err(), "missing target should make chmod fail");
    assert_eq!(
        fs::read(archive_path).unwrap(),
        original,
        "failed chmod must not persist partial changes"
    );
    assert_eq!(archive::entry_mode(archive_path, ENTRY_PATH), 0o777);
}
