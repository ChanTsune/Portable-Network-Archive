use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path};

fn init_resource<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    let parent_path = path.parent().unwrap_or_else(|| Path::new("."));
    let base_path = parent_path.join("in");
    let base_path = &base_path;
    if base_path.exists() {
        fs::remove_dir_all(base_path).unwrap();
    }
    fs::create_dir_all(base_path).unwrap();
    fs::write(base_path.join("origin.txt"), b"abc").unwrap();
    fs::hard_link(base_path.join("origin.txt"), base_path.join("link.txt")).unwrap();
    fs::create_dir_all(base_path.join("origin")).unwrap();
    fs::write(base_path.join("origin/origin2.txt"), b"def").unwrap();
    fs::create_dir_all(base_path.join("link")).unwrap();
    fs::hard_link(
        base_path.join("origin/origin2.txt"),
        base_path.join("link/link2.txt"),
    )
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &path.to_string_lossy(),
        "--overwrite",
        &base_path.to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();
}

#[test]
fn hardlink() {
    setup();
    init_resource("hardlink/hardlink.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "hardlink/hardlink.pna",
        "--overwrite",
        "--out-dir",
        "hardlink/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        "abc",
        fs::read_to_string("hardlink/dist/hardlink/in/origin.txt").unwrap()
    );

    assert_eq!(
        "abc",
        fs::read_to_string("hardlink/dist/hardlink/in/link.txt").unwrap()
    );

    assert_eq!(
        "def",
        fs::read_to_string("hardlink/dist/hardlink/in/origin/origin2.txt").unwrap()
    );
    assert_eq!(
        "def",
        fs::read_to_string("hardlink/dist/hardlink/in/link/link2.txt").unwrap()
    );
}
