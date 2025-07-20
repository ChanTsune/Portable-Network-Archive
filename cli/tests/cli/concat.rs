use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn concat_archive() {
    setup();
    TestResources::extract_in("raw/", "concat_archive/in").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
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
        "concat_archive/concatenated.pna",
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
    diff("concat_archive/in", "concat_archive/out").unwrap();
}
