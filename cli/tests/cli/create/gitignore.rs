use crate::utils::{diff::diff, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_with_gitignore() {
    setup();
    fs::create_dir_all("gitignore/source").unwrap();
    fs::write("gitignore/source/.gitignore", "*.log\n").unwrap();
    fs::write("gitignore/source/keep.txt", b"text").unwrap();
    fs::write("gitignore/source/skip.log", b"log").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "gitignore/gitignore.pna",
        "--overwrite",
        "gitignore/source",
        "--gitignore",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "gitignore/gitignore.pna",
        "--overwrite",
        "--out-dir",
        "gitignore/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::remove_file("gitignore/source/skip.log").unwrap();
    diff("gitignore/source/", "gitignore/out/").unwrap();
}
