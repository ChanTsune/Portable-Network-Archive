#[cfg(not(target_family = "wasm"))]
mod option_overwrite;

use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: Split archive parts exist.
/// Action: Concatenate split archive parts into a single archive.
/// Expectation: Extracted content matches original.
#[test]
fn concat_archive() {
    setup();
    TestResources::extract_in("raw/", "concat_archive/in").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "-f",
        "concat_archive/concat.pna",
        "--overwrite",
        "concat_archive/in",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "split",
        "-f",
        "concat_archive/concat.pna",
        "--overwrite",
        "--max-size",
        "100kb",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "concat",
        "-f",
        "concat_archive/concatenated.pna",
        "-f",
        "concat_archive/concat.part1.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "concat_archive/concatenated.pna",
        "--overwrite",
        "--out-dir",
        "concat_archive/out",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert_dirs_equal("concat_archive/in", "concat_archive/out");
}
