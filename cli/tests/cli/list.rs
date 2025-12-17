#[cfg(not(target_family = "wasm"))]
mod exclude_vcs;
mod missing_file;
#[cfg(not(target_family = "wasm"))]
mod option_format_bsdtar;
#[cfg(not(target_family = "wasm"))]
mod option_format_jsonl;
#[cfg(not(target_family = "wasm"))]
mod option_format_line;
#[cfg(not(target_family = "wasm"))]
mod option_format_tree;

use crate::utils::{EmbedExt, TestResources, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn archive_list_solid() {
    setup();
    TestResources::extract_in("raw/", "list_solid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "list_solid/list_solid.pna",
        "--overwrite",
        "list_solid/in/",
        "--solid",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from(["pna", "list", "list_solid/list_solid.pna", "--solid"])
        .unwrap()
        .execute()
        .unwrap();
}

#[test]
fn archive_list_detail() {
    setup();
    TestResources::extract_in("raw/", "list_detail/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "list_detail/list_detail.pna",
        "--overwrite",
        "list_detail/in/",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "list",
        "-l",
        "list_detail/list_detail.pna",
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();
}

#[test]
fn archive_list_solid_detail() {
    setup();
    TestResources::extract_in("raw/", "list_solid_detail/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "list_solid_detail/list_solid_detail.pna",
        "--overwrite",
        "list_solid_detail/in/",
        "--solid",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "list",
        "-l",
        "list_solid_detail/list_solid_detail.pna",
        "--solid",
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();
}
