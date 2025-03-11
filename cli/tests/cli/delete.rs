mod exclude;

use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn delete_overwrite() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_overwrite/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_overwrite/delete_overwrite.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_overwrite/in/"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_overwrite/delete_overwrite.pna"
        ),
        "**/raw/empty.txt",
    ]))
    .unwrap();
    fs::remove_file(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/delete_overwrite/in/raw/empty.txt"
    ))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_overwrite/delete_overwrite.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_overwrite/out/"),
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_overwrite/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_overwrite/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_overwrite/out/"),
    )
    .unwrap();
}

#[test]
fn delete_output() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_output/delete_output.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/in/"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_output/delete_output.pna"
        ),
        "**/raw/text.txt",
        "--output",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/deleted.pna"),
    ]))
    .unwrap();
    fs::remove_file(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/delete_output/in/raw/text.txt"
    ))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/deleted.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_output/out/"),
    )
    .unwrap();
}

#[test]
fn delete_solid() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_solid/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_solid/delete_solid.pna"
        ),
        "--overwrite",
        "--solid",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_solid/in/"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_solid/delete_solid.pna"
        ),
        "**/raw/text.txt",
    ]))
    .unwrap();
    fs::remove_file(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/delete_solid/in/raw/text.txt"
    ))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_solid/delete_solid.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_solid/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_solid/in/")).to_string(),
    ]))
    .unwrap();
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_solid/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_solid/out/"),
    )
    .unwrap();
}

#[test]
fn delete_unsolid() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_unsolid/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_unsolid/delete_unsolid.pna"
        ),
        "--overwrite",
        "--solid",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_unsolid/in/"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "--unsolid",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_unsolid/delete_unsolid.pna"
        ),
        "**/raw/text.txt",
    ]))
    .unwrap();
    fs::remove_file(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/delete_unsolid/in/raw/text.txt"
    ))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/delete_unsolid/delete_unsolid.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_unsolid/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_unsolid/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_unsolid/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/delete_unsolid/out/"),
    )
    .unwrap();
}
