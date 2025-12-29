use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: Archive contains entries with a common path prefix.
/// Action: Extract with `-s` substitution rule that strips the prefix.
/// Expectation: Entries are extracted at transformed paths with prefix removed.
#[test]
fn extract_with_substitution() {
    setup();
    TestResources::extract_in("zstd.pna", "extract_with_substitution/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_substitution/zstd.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_substitution/out/",
        "-s",
        "#raw/##",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify original paths with prefix do NOT exist (substitution removed them)
    assert!(
        !fs::exists("extract_with_substitution/out/raw/text.txt").unwrap(),
        "Original prefixed path should not exist after substitution"
    );

    // Verify transformed paths DO exist (prefix was stripped)
    assert!(
        fs::exists("extract_with_substitution/out/text.txt").unwrap(),
        "Transformed path should exist"
    );
    assert!(
        fs::exists("extract_with_substitution/out/images/icon.png").unwrap(),
        "Transformed nested path should exist"
    );
}
