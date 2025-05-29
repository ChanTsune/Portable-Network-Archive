mod exclude;
mod exclude_from;
mod files_from;
mod files_from_stdin;
mod include;
mod password;
mod password_file;

use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_overwrite() {
    setup();
    TestResources::extract_in("raw/", "delete_overwrite/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_overwrite/delete_overwrite.pna",
        "--overwrite",
        "delete_overwrite/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_overwrite/delete_overwrite.pna",
        "**/raw/empty.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_overwrite/in/raw/empty.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_overwrite/delete_overwrite.pna",
        "--overwrite",
        "--out-dir",
        "delete_overwrite/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("delete_overwrite/in/", "delete_overwrite/out/").unwrap();
}

#[test]
fn delete_output() {
    setup();
    TestResources::extract_in("raw/", "delete_output/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_output/delete_output.pna",
        "--overwrite",
        "delete_output/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_output/delete_output.pna",
        "**/raw/text.txt",
        "--output",
        "delete_output/deleted.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_output/in/raw/text.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_output/deleted.pna",
        "--overwrite",
        "--out-dir",
        "delete_output/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("delete_output/in/", "delete_output/out/").unwrap();
}

#[test]
fn delete_solid() {
    setup();
    TestResources::extract_in("raw/", "delete_solid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_solid/delete_solid.pna",
        "--overwrite",
        "--solid",
        "delete_solid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_solid/delete_solid.pna",
        "**/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_solid/in/raw/text.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_solid/delete_solid.pna",
        "--overwrite",
        "--out-dir",
        "delete_solid/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff("delete_solid/in/", "delete_solid/out/").unwrap();
}

#[test]
fn delete_unsolid() {
    setup();
    TestResources::extract_in("raw/", "delete_unsolid/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_unsolid/delete_unsolid.pna",
        "--overwrite",
        "--solid",
        "delete_unsolid/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "--unsolid",
        "delete_unsolid/delete_unsolid.pna",
        "**/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    fs::remove_file("delete_unsolid/in/raw/text.txt").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_unsolid/delete_unsolid.pna",
        "--overwrite",
        "--out-dir",
        "delete_unsolid/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("delete_unsolid/in/", "delete_unsolid/out/").unwrap();
}
