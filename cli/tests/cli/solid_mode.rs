use crate::utils::{EmbedExt, TestResources, diff::assert_dirs_equal, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn solid_store_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_store/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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
    assert_dirs_equal("solid_store/in/", "solid_store/out/");
}

#[test]
fn solid_zstd_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_zstd/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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

    assert_dirs_equal("solid_zstd/in/", "solid_zstd/out/");
}

#[test]
fn solid_xz_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_xz/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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

    assert_dirs_equal("solid_xz/in/", "solid_xz/out/");
}

#[test]
fn solid_deflate_archive() {
    setup();
    TestResources::extract_in("raw/", "solid_deflate/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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
    assert_dirs_equal("solid_deflate/in/", "solid_deflate/out/");
}
