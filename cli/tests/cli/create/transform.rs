use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_with_transform() {
    setup();
    TestResources::extract_in("raw/", "create_with_transform/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_transform/create_with_transform.pna",
        "--overwrite",
        "create_with_transform/in/",
        "--transform",
        "s,create_with_transform/in/,,",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("create_with_transform/create_with_transform.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_transform/create_with_transform.pna",
        "--overwrite",
        "--out-dir",
        "create_with_transform/out/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("create_with_transform/in/", "create_with_transform/out/").unwrap();
}
