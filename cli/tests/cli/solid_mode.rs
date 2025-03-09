use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn solid_store_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/solid_store.pna"),
        "--store",
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/in/"),
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/solid_store.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/out/")).to_string(),
    ]))
    .unwrap();
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_store/out/"),
    )
    .unwrap();
}

#[test]
fn solid_zstd_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/solid_zstd.pna"),
        "--zstd",
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/in/"),
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/solid_zstd.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_zstd/out/"),
    )
    .unwrap();
}

#[test]
fn solid_xz_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/solid_xz.pna"),
        "--xz",
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/in/"),
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/solid_xz.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_xz/out/"),
    )
    .unwrap();
}

#[test]
fn solid_deflate_archive() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_deflate/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/solid_deflate/solid_deflate.pna"
        ),
        "--deflate",
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_deflate/in/"),
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/solid_deflate/solid_deflate.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_deflate/out/"),
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_deflate/in/")).to_string(),
    ]))
    .unwrap();
    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_deflate/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/solid_deflate/out/"),
    )
    .unwrap();
}
