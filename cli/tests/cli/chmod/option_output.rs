use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive contains a file with permission 0o777.
/// Action: Run `pna experimental chmod` with `--output` to a new path.
/// Expectation: The output archive has the updated mode; the original is untouched.
#[test]
fn chmod_output() {
    setup();
    let _ = std::fs::remove_file("chmod_output_out.pna");

    archive::create_archive_with_permissions(
        "chmod_output.pna",
        &[FileEntryDef {
            path: "test.txt",
            content: b"test content",
            permission: 0o777,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_output.pna",
        "--output",
        "chmod_output_out.pna",
        "--",
        "-x",
        "test.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::modes_by_entry("chmod_output.pna"),
        vec![("test.txt".to_string(), Some(0o777))]
    );
    assert_eq!(
        archive::modes_by_entry("chmod_output_out.pna"),
        vec![("test.txt".to_string(), Some(0o666))]
    );
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna experimental chmod` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn chmod_output_without_overwrite_refuses_to_clobber() {
    setup();
    archive::create_archive_with_permissions(
        "chmod_overwrite.pna",
        &[FileEntryDef {
            path: "test.txt",
            content: b"test content",
            permission: 0o777,
        }],
    )
    .unwrap();
    fs::write("chmod_overwrite_out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_overwrite.pna",
        "--output",
        "chmod_overwrite_out.pna",
        "--",
        "-x",
        "test.txt",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("chmod_overwrite_out.pna").unwrap(), b"sentinel");
    assert_eq!(
        archive::modes_by_entry("chmod_overwrite.pna"),
        vec![("test.txt".to_string(), Some(0o777))]
    );
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same chmod with `--overwrite`.
/// Expectation: The output holds the updated mode; the original is untouched.
#[test]
fn chmod_output_with_overwrite_replaces() {
    setup();
    archive::create_archive_with_permissions(
        "chmod_overwrite_ok.pna",
        &[FileEntryDef {
            path: "test.txt",
            content: b"test content",
            permission: 0o777,
        }],
    )
    .unwrap();
    fs::write("chmod_overwrite_ok_out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod_overwrite_ok.pna",
        "--output",
        "chmod_overwrite_ok_out.pna",
        "--overwrite",
        "--",
        "-x",
        "test.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::modes_by_entry("chmod_overwrite_ok.pna"),
        vec![("test.txt".to_string(), Some(0o777))]
    );
    assert_eq!(
        archive::modes_by_entry("chmod_overwrite_ok_out.pna"),
        vec![("test.txt".to_string(), Some(0o666))]
    );
}
