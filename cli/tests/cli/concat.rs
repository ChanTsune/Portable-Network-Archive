use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn concat_archive() {
    setup();
    TestResources::extract_in("raw/", "concat_archive/in").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "create",
        "concat_archive/concat.pna",
        "--overwrite",
        "concat_archive/in",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "split",
        "concat_archive/concat.pna",
        "--overwrite",
        "--max-size",
        "100kb",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "concat",
        "concat_archive/concatenated.pna",
        "concat_archive/concat.part1.pna",
        "--overwrite",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "concat_archive/concatenated.pna",
        "--overwrite",
        "--out-dir",
        "concat_archive/out",
        "--strip-components",
        &components_count("concat_archive/in").to_string(),
    ]))
    .unwrap();
    diff("concat_archive/in", "concat_archive/out").unwrap();
}
