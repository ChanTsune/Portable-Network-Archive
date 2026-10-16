use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna strip` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn strip_output_without_overwrite_refuses_to_clobber() {
    setup();
    let path = "strip_overwrite.pna";
    archive::create_archive_with_permissions(
        path,
        &[FileEntryDef {
            path: "strip.txt",
            content: b"strip",
            permission: 0o644,
        }],
    )
    .unwrap();
    fs::write("strip_overwrite_out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "-f",
        path,
        "--output",
        "strip_overwrite_out.pna",
        "strip.txt",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("strip_overwrite_out.pna").unwrap(), b"sentinel");
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same strip with `--overwrite`.
/// Expectation: The output is replaced; the original is untouched.
#[test]
fn strip_output_with_overwrite_replaces() {
    setup();
    let path = "strip_overwrite_ok.pna";
    archive::create_archive_with_permissions(
        path,
        &[FileEntryDef {
            path: "strip.txt",
            content: b"strip",
            permission: 0o644,
        }],
    )
    .unwrap();
    fs::write("strip_overwrite_ok_out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "-f",
        path,
        "--output",
        "strip_overwrite_ok_out.pna",
        "--overwrite",
        "strip.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_ne!(fs::read("strip_overwrite_ok_out.pna").unwrap(), b"sentinel");
    let mut entries = Vec::new();
    archive::for_each_entry(path, |entry| {
        entries.push(entry.metadata().permission_mode().map(|m| m.get()));
    })
    .unwrap();
    assert_eq!(entries, [Some(0o644)]);
}
