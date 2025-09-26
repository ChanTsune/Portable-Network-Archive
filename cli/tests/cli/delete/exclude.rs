use crate::utils::{diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_output_exclude() {
    setup();
    TestResources::extract_in("raw/", "delete_output_exclude/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_output_exclude/delete_output_exclude.pna",
        "--overwrite",
        "delete_output_exclude/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_output_exclude/delete_output_exclude.pna",
        "**.pna",
        "--exclude",
        "**/empty.*",
        "--unstable",
        "--output",
        "delete_output_exclude/delete_excluded.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_output_exclude/delete_excluded.pna",
        "--overwrite",
        "--out-dir",
        "delete_output_exclude/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::remove_file("delete_output_exclude/in/raw/pna/nest.pna").unwrap();

    diff("delete_output_exclude/in/", "delete_output_exclude/out/").unwrap();
}
