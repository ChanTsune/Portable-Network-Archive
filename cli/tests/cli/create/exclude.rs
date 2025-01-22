use crate::utils;
use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn create_with_exclude() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_with_exclude/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/create_with_exclude.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_with_exclude/in/"),
        "--exclude",
        "**.txt",
        "--unstable",
    ]))
    .unwrap();
    assert!(fs::exists(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/create_with_exclude/create_with_exclude.pna"
    ))
    .unwrap());

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/create_with_exclude.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_with_exclude/out/"),
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    // Remove files that are expected to be excluded from input for comparison
    let expected_to_be_excluded = [
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/in/raw/first/second/third/pna.txt"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/in/raw/parent/child.txt"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/in/raw/empty.txt"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/create_with_exclude/in/raw/text.txt"
        ),
    ];
    for file in expected_to_be_excluded {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_with_exclude/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/create_with_exclude/out/"),
    )
    .unwrap();
}
