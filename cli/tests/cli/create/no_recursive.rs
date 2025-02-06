use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn no_recursive() {
    setup();
    TestResources::extract_in("raw/", "no_recursive/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "no_recursive/no_recursive.pna",
        "--overwrite",
        "--no-recursive",
        "no_recursive/in/",
    ]))
    .unwrap();
    assert!(fs::exists("no_recursive/no_recursive.pna").unwrap());

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "no_recursive/no_recursive.pna",
        "--overwrite",
        "--out-dir",
        "no_recursive/out/",
        "--strip-components",
        "2",
    ]))
    .unwrap();
    fs::create_dir_all("no_recursive/out/").unwrap();
    assert_eq!(fs::read_dir("no_recursive/out/").unwrap().count(), 0);
}
