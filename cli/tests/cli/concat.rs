use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn concat_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/in"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "create",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/concat.pna"),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/in"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "split",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/concat.pna"),
        "--overwrite",
        "--max-size",
        "100kb",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "concat",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/concat_archive/concatenated.pna"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/concat_archive/concat.part1.pna"
        ),
        "--overwrite",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/concat_archive/concatenated.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/out"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/in")).to_string(),
    ]))
    .unwrap();
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/in"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/concat_archive/out"),
    )
    .unwrap();
}
