use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn multipart_archive() {
    setup();
    TestResources::extract_in("multipart_test.txt", "./multipart_archive/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "./multipart_archive/multipart.pna",
        "--overwrite",
        "./multipart_archive/in/multipart_test.txt",
        "--unstable",
        "--split",
        "110",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "./multipart_archive/multipart.part1.pna",
        "--overwrite",
        "--out-dir",
        "./multipart_archive/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_dirs_equal("./multipart_archive/in/", "./multipart_archive/out/");
}

/// Reads every part of the split archive named `base`, concatenated in order.
fn concatenated_parts(base: &str) -> Vec<u8> {
    let mut parts = 0;
    let mut bytes = Vec::new();
    while let Ok(part) = fs::read(format!("{base}.part{}.pna", parts + 1)) {
        bytes.extend(part);
        parts += 1;
    }
    assert!(parts > 1, "expected {base} to be split into several parts");
    bytes
}

#[test]
fn multipart_archive_joined_into_single_file() {
    setup();
    TestResources::extract_in("multipart_test.txt", "./multipart_joined/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "./multipart_joined/multipart.pna",
        "--overwrite",
        "./multipart_joined/in/multipart_test.txt",
        "--unstable",
        "--split",
        "110",
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::write(
        "./multipart_joined/joined.pna",
        concatenated_parts("./multipart_joined/multipart"),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "./multipart_joined/joined.pna",
        "--overwrite",
        "--out-dir",
        "./multipart_joined/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_dirs_equal("./multipart_joined/in/", "./multipart_joined/out/");
}
