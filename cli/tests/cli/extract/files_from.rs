use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive contains multiple resources under `raw/`.
/// Action: Create the archive via `pna create`, then extract it with
///         `pna extract --files-from <manifest> --out-dir <dir>`.
/// Expectation: Only the manifest-listed entries are materialized; everything else remains absent.
#[test]
fn extract_with_files_from_manifest() {
    setup();
    TestResources::extract_in("raw/", "extract_files_from/in/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_files_from/extract_files_from.pna",
        "--overwrite",
        "extract_files_from/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let manifest = "extract_files_from/list.txt";
    fs::write(
        manifest,
        [
            "extract_files_from/in/raw/images/icon.png",
            "extract_files_from/in/raw/text.txt",
        ]
        .join("\n"),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_files_from/extract_files_from.pna",
        "--overwrite",
        "--out-dir",
        "extract_files_from/out/",
        "--files-from",
        manifest,
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(
        fs::exists("extract_files_from/out/extract_files_from/in/raw/images/icon.png").unwrap()
    );
    assert!(fs::exists("extract_files_from/out/extract_files_from/in/raw/text.txt").unwrap());
    assert!(
        !fs::exists("extract_files_from/out/extract_files_from/in/raw/images/icon.svg").unwrap()
    );
    assert!(!fs::exists("extract_files_from/out/extract_files_from/in/raw/pna/empty.pna").unwrap());

    diff(
        "extract_files_from/in/raw/images/icon.png",
        "extract_files_from/out/extract_files_from/in/raw/images/icon.png",
    )
    .unwrap();
    diff(
        "extract_files_from/in/raw/text.txt",
        "extract_files_from/out/extract_files_from/in/raw/text.txt",
    )
    .unwrap();
}
