#[cfg(not(target_family = "wasm"))]
mod dump;
#[cfg(not(target_family = "wasm"))]
mod restore;

use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_acl_get_set() {
    setup();
    TestResources::extract_in("raw/", "acl_get_set/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "acl_get_set/acl_get_set.pna",
        "--overwrite",
        "acl_get_set/in/",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
        "-m",
        "g:test_group:r,w,x",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
        "-x",
        "g:test_group",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "get",
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "acl_get_set/acl_get_set.pna",
        "--overwrite",
        "--out-dir",
        "acl_get_set/out/",
        "--strip-components",
        &components_count("acl_get_set/in/").to_string(),
    ]))
    .unwrap();

    diff("acl_get_set/in/", "acl_get_set/out/").unwrap();
}
