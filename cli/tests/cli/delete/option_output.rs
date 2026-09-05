use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries from the archive
///         by `pna experimental delete` with `--output`.
/// Expectation: Removes all entries that match the given patterns from the archive
///              and creates a new archive file with the result.
#[test]
fn delete_output() {
    setup();
    let _ = std::fs::remove_file("delete_output/deleted.pna");
    TestResources::extract_in("raw/", "delete_output/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "delete_output/delete_output.pna",
        "--overwrite",
        "--no-keep-dir",
        "delete_output/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "delete",
        "-f",
        "delete_output/delete_output.pna",
        "**/raw/text.txt",
        "--output",
        "delete_output/deleted.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_output/delete_output.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    for required in [
        "delete_output/in/raw/empty.txt",
        "delete_output/in/raw/first/second/third/pna.txt",
        "delete_output/in/raw/text.txt",
        "delete_output/in/raw/parent/child.txt",
        "delete_output/in/raw/pna/empty.pna",
        "delete_output/in/raw/pna/nest.pna",
        "delete_output/in/raw/images/icon.svg",
        "delete_output/in/raw/images/icon.png",
        "delete_output/in/raw/images/icon.bmp",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_output/deleted.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    for required in [
        "delete_output/in/raw/images/icon.png",
        "delete_output/in/raw/images/icon.svg",
        "delete_output/in/raw/pna/empty.pna",
        "delete_output/in/raw/parent/child.txt",
        "delete_output/in/raw/pna/nest.pna",
        "delete_output/in/raw/empty.txt",
        "delete_output/in/raw/images/icon.bmp",
        "delete_output/in/raw/first/second/third/pna.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna experimental delete` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn delete_output_without_overwrite_refuses_to_clobber() {
    setup();
    fs::create_dir_all("delete_ow_guard/in").unwrap();
    fs::write("delete_ow_guard/in/file.txt", b"data").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "delete_ow_guard/archive.pna",
        "--overwrite",
        "delete_ow_guard/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::write("delete_ow_guard/out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "delete",
        "-f",
        "delete_ow_guard/archive.pna",
        "delete_ow_guard/in/file.txt",
        "--output",
        "delete_ow_guard/out.pna",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("delete_ow_guard/out.pna").unwrap(), b"sentinel");
    let mut seen = HashSet::new();
    archive::for_each_entry("delete_ow_guard/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();
    assert!(!seen.is_empty(), "original archive must be untouched");
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same delete with `--overwrite`.
/// Expectation: The output is replaced with the filtered archive; the original is untouched.
#[test]
fn delete_output_with_overwrite_replaces() {
    setup();
    fs::create_dir_all("delete_ow_guard_ok/in").unwrap();
    fs::write("delete_ow_guard_ok/in/file.txt", b"data").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "delete_ow_guard_ok/archive.pna",
        "--overwrite",
        "delete_ow_guard_ok/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::write("delete_ow_guard_ok/out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "delete",
        "-f",
        "delete_ow_guard_ok/archive.pna",
        "delete_ow_guard_ok/in/file.txt",
        "--output",
        "delete_ow_guard_ok/out.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_ne!(fs::read("delete_ow_guard_ok/out.pna").unwrap(), b"sentinel");
}

/// Precondition: An archive exists.
/// Action: Run `pna experimental delete` with `--output` naming the input itself.
/// Expectation: Without `--overwrite` the command fails even though the destination
/// is the in-place target; with `--overwrite` it rewrites the input.
#[test]
fn delete_output_naming_the_input_requires_overwrite() {
    setup();
    fs::create_dir_all("delete_ow_guard_self/in").unwrap();
    fs::write("delete_ow_guard_self/in/file.txt", b"data").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "delete_ow_guard_self/archive.pna",
        "--overwrite",
        "delete_ow_guard_self/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "delete",
        "-f",
        "delete_ow_guard_self/archive.pna",
        "delete_ow_guard_self/in/file.txt",
        "--output",
        "delete_ow_guard_self/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap_err();
    assert!(format!("{error:?}").contains("already exists"));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "delete",
        "-f",
        "delete_ow_guard_self/archive.pna",
        "delete_ow_guard_self/in/file.txt",
        "--output",
        "delete_ow_guard_self/archive.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();
}
