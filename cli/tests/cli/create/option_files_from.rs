use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

#[test]
fn create_with_files_from() {
    setup();
    TestResources::extract_in("raw/", "create_with_files_from/src/").unwrap();

    let list_path = "create_with_files_from/files.txt";
    fs::write(
        list_path,
        [
            "create_with_files_from/src/raw/empty.txt",
            "create_with_files_from/src/raw/text.txt",
        ]
        .join("\n"),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_files_from/create_with_files_from.pna",
        "--overwrite",
        "--files-from",
        list_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_with_files_from/create_with_files_from.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "create_with_files_from/src/raw/empty.txt",
        "create_with_files_from/src/raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_files_from_crlf() {
    setup();
    TestResources::extract_in("raw/", "create_with_files_from_crlf/src/").unwrap();

    // Use CRLF line endings (Windows-style)
    let list_path = "create_with_files_from_crlf/files.txt";
    fs::write(
        list_path,
        "create_with_files_from_crlf/src/raw/empty.txt\r\ncreate_with_files_from_crlf/src/raw/text.txt\r\n",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_files_from_crlf/test.pna",
        "--overwrite",
        "--files-from",
        list_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_with_files_from_crlf/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "create_with_files_from_crlf/src/raw/empty.txt",
        "create_with_files_from_crlf/src/raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

#[test]
fn create_with_files_from_cr() {
    setup();
    TestResources::extract_in("raw/", "create_with_files_from_cr/src/").unwrap();

    // Use CR line endings (old Mac-style)
    let list_path = "create_with_files_from_cr/files.txt";
    fs::write(
        list_path,
        "create_with_files_from_cr/src/raw/empty.txt\rcreate_with_files_from_cr/src/raw/text.txt\r",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_with_files_from_cr/test.pna",
        "--overwrite",
        "--files-from",
        list_path,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_with_files_from_cr/test.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "create_with_files_from_cr/src/raw/empty.txt",
        "create_with_files_from_cr/src/raw/text.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
