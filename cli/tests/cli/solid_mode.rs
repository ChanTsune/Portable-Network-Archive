use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn solid_store_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_store/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_store/solid_store.pna",
        "--store",
        "--overwrite",
        "solid_store/in/",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_store/solid_store.pna",
        "--overwrite",
        "--out-dir",
        "solid_store/out/",
        "--strip-components",
        &components_count("solid_store/out/").to_string(),
    ]))
    .unwrap();
    diff("solid_store/in/", "solid_store/out/").unwrap();
}

#[test]
fn solid_zstd_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_zstd/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_zstd/solid_zstd.pna",
        "--zstd",
        "--overwrite",
        "solid_zstd/in/",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_zstd/solid_zstd.pna",
        "--overwrite",
        "--out-dir",
        "solid_zstd/out/",
        "--strip-components",
        &components_count("solid_zstd/in/").to_string(),
    ]))
    .unwrap();

    diff("solid_zstd/in/", "solid_zstd/out/").unwrap();
}

#[test]
fn solid_xz_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_xz/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_xz/solid_xz.pna",
        "--xz",
        "--overwrite",
        "solid_xz/in/",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_xz/solid_xz.pna",
        "--overwrite",
        "--out-dir",
        "solid_xz/out/",
        "--strip-components",
        &components_count("solid_xz/in/").to_string(),
    ]))
    .unwrap();

    diff("solid_xz/in/", "solid_xz/out/").unwrap();
}

#[test]
fn solid_deflate_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_deflate/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "solid_deflate/solid_deflate.pna",
        "--deflate",
        "--overwrite",
        "solid_deflate/in/",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "solid_deflate/solid_deflate.pna",
        "--overwrite",
        "--out-dir",
        "solid_deflate/out/",
        "--strip-components",
        &components_count("solid_deflate/in/").to_string(),
    ]))
    .unwrap();
    diff("solid_deflate/in/", "solid_deflate/out/").unwrap();
}
