use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn append_exclude() {
    setup();
    TestResources::extract_in("raw/", "append_exclude/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "append_exclude/append.pna",
        "--overwrite",
        "append_exclude/in/",
        "--exclude",
        "*/extra/*",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Copy extra input
    TestResources::extract_in("store.pna", "append_exclude/in/extra/").unwrap();
    TestResources::extract_in("zstd.pna", "append_exclude/in/extra/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "-f",
        "append_exclude/append.pna",
        "append_exclude/in/extra/",
        "--exclude",
        "*/z*.pna",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "append_exclude/append.pna",
        "--overwrite",
        "--out-dir",
        "append_exclude/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // Verify common subtree matches
    assert_dirs_equal("append_exclude/in/raw/", "append_exclude/out/raw/");
    // Verify excluded file is absent, non-excluded file is present
    assert!(std::fs::exists("append_exclude/out/extra/store.pna").unwrap());
    assert!(!std::fs::exists("append_exclude/out/extra/zstd.pna").unwrap());
}
