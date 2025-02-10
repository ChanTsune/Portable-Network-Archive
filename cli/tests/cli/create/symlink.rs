use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};
use std::{
    fs,
    io::prelude::*,
    path::{Path, PathBuf},
};

fn init_resource<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref();
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();
    let mut file = fs::File::create(dir.join("text.txt")).unwrap();
    file.write_all(b"content").unwrap();
    pna::fs::symlink(Path::new("text.txt"), dir.join("link.txt")).unwrap();
}

#[test]
fn symlink_no_follow() {
    setup();
    init_resource("symlink_no_follow/source");
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_no_follow/symlink_no_follow.pna",
        "--overwrite",
        "symlink_no_follow/source",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_no_follow/symlink_no_follow.pna",
        "--overwrite",
        "--out-dir",
        "symlink_no_follow/dist",
        "--strip-components",
        "2",
    ]))
    .unwrap();

    assert!(PathBuf::from("symlink_no_follow/dist/link.txt").is_symlink());
}
