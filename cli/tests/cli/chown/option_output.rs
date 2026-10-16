use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

fn uname_of(path: &str) -> String {
    let mut uname = String::new();
    archive::for_each_entry(path, |entry| {
        uname = entry.metadata().owner_user_name().unwrap().to_string();
    })
    .unwrap();
    uname
}

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` with `--output` to a new path.
/// Expectation: The output archive has the updated owner; the original is untouched.
#[test]
fn chown_output() {
    setup();
    let _ = std::fs::remove_file("chown_output_out.pna");

    archive::create_archive_with_permissions(
        "chown_output.pna",
        &[FileEntryDef {
            path: "target.txt",
            content: b"target",
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_output.pna",
        "--output",
        "chown_output_out.pna",
        "new_user",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let uname_of = |path: &str| {
        let mut uname = String::new();
        archive::for_each_entry(path, |entry| {
            uname = entry.metadata().owner_user_name().unwrap().to_string();
        })
        .unwrap();
        uname
    };

    assert_eq!(uname_of("chown_output.pna"), "user");
    assert_eq!(uname_of("chown_output_out.pna"), "new_user");
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna experimental chown` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn chown_output_without_overwrite_refuses_to_clobber() {
    setup();
    archive::create_archive_with_permissions(
        "chown_overwrite.pna",
        &[FileEntryDef {
            path: "target.txt",
            content: b"target",
            permission: 0o644,
        }],
    )
    .unwrap();
    fs::write("chown_overwrite_out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_overwrite.pna",
        "--output",
        "chown_overwrite_out.pna",
        "new_user",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("chown_overwrite_out.pna").unwrap(), b"sentinel");
    assert_eq!(uname_of("chown_overwrite.pna"), "user");
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same chown with `--overwrite`.
/// Expectation: The output holds the updated owner; the original is untouched.
#[test]
fn chown_output_with_overwrite_replaces() {
    setup();
    archive::create_archive_with_permissions(
        "chown_overwrite_ok.pna",
        &[FileEntryDef {
            path: "target.txt",
            content: b"target",
            permission: 0o644,
        }],
    )
    .unwrap();
    fs::write("chown_overwrite_ok_out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_overwrite_ok.pna",
        "--output",
        "chown_overwrite_ok_out.pna",
        "--overwrite",
        "new_user",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(uname_of("chown_overwrite_ok.pna"), "user");
    assert_eq!(uname_of("chown_overwrite_ok_out.pna"), "new_user");
}
