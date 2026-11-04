use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive holds entries, and one named operand matches none of them.
/// Action: Run `pna strip` naming an existing and a missing entry.
/// Expectation: The command fails and the archive is left byte-for-byte unchanged.
#[test]
fn fail_with_missing_file() {
    setup();
    let path = "strip_missing_file.pna";
    archive::create_archive_with_permissions(
        path,
        &[FileEntryDef {
            path: "present.txt",
            content: b"present",
            permission: 0o644,
        }],
    )
    .unwrap();
    let before = fs::read(path).unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "-f",
        path,
        "present.txt",
        "not_found.txt",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(
        fs::read(path).unwrap(),
        before,
        "the archive must be left unchanged"
    );
}
