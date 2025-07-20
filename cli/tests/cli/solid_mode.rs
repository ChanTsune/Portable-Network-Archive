use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn solid_store_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_store/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_store/solid_store.pna",
        "--store",
        "--overwrite",
        "solid_store/in/",
        "--solid",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_store/solid_store.pna",
        "--overwrite",
        "--out-dir",
        "solid_store/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff("solid_store/in/", "solid_store/out/").unwrap();
}

#[test]
fn solid_zstd_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_zstd/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_zstd/solid_zstd.pna",
        "--zstd",
        "--overwrite",
        "solid_zstd/in/",
        "--solid",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_zstd/solid_zstd.pna",
        "--overwrite",
        "--out-dir",
        "solid_zstd/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("solid_zstd/in/", "solid_zstd/out/").unwrap();
}

#[test]
fn solid_xz_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_xz/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_xz/solid_xz.pna",
        "--xz",
        "--overwrite",
        "solid_xz/in/",
        "--solid",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_xz/solid_xz.pna",
        "--overwrite",
        "--out-dir",
        "solid_xz/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("solid_xz/in/", "solid_xz/out/").unwrap();
}

#[test]
fn solid_deflate_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_deflate/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_deflate/solid_deflate.pna",
        "--deflate",
        "--overwrite",
        "solid_deflate/in/",
        "--solid",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_deflate/solid_deflate.pna",
        "--overwrite",
        "--out-dir",
        "solid_deflate/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff("solid_deflate/in/", "solid_deflate/out/").unwrap();
}
