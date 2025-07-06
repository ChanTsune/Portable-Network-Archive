use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn init_resource<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref();
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();

    fs::write(dir.join("text.txt"), b"content").unwrap();
    pna::fs::symlink(Path::new("text.txt"), dir.join("link.txt")).unwrap();

    fs::create_dir_all(dir.join("dir")).unwrap();
    fs::write(dir.join("dir/in_dir_text.txt"), b"dir_content").unwrap();
    pna::fs::symlink(Path::new("dir"), dir.join("link_dir")).unwrap();
}

#[test]
fn symlink_no_follow() {
    setup();
    init_resource("symlink_no_follow/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_no_follow/symlink_no_follow.pna",
        "--overwrite",
        "--keep-dir",
        "symlink_no_follow/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "symlink_no_follow/symlink_no_follow.pna",
        |entry| match entry.header().path().as_str() {
            "symlink_no_follow/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_no_follow/source/text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_no_follow/source/dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_no_follow/source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_no_follow/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
            }
            "symlink_no_follow/source/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_no_follow/symlink_no_follow.pna",
        "--overwrite",
        "--out-dir",
        "symlink_no_follow/dist",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(PathBuf::from("symlink_no_follow/dist/link.txt").is_symlink());
    assert!(PathBuf::from("symlink_no_follow/dist/link_dir").is_symlink());
    assert_eq!(
        fs::read_to_string("symlink_no_follow/dist/dir/in_dir_text.txt").unwrap(),
        fs::read_to_string("symlink_no_follow/dist/link_dir/in_dir_text.txt").unwrap(),
    );
}

// FIXME: On Github Actions Windows runner disabled due to insufficient privileges for execution
#[cfg(unix)]
#[test]
fn symlink_follow() {
    setup();
    init_resource("symlink_follow/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_follow/symlink_follow.pna",
        "--overwrite",
        "--keep-dir",
        "--follow-links",
        "symlink_follow/source",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("symlink_follow/symlink_follow.pna", |entry| {
        match entry.header().path().as_str() {
            "symlink_follow/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_follow/source/text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_follow/source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_follow/source/link_dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            path => unreachable!("unexpected entry found: {path}"),
        }
    })
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_follow/symlink_follow.pna",
        "--overwrite",
        "--out-dir",
        "symlink_follow/dist",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(!PathBuf::from("symlink_follow/dist/link.txt").is_symlink());
    assert!(!PathBuf::from("symlink_follow/dist/link_dir").is_symlink());
    assert_eq!(
        fs::read_to_string("symlink_follow/dist/dir/in_dir_text.txt").unwrap(),
        fs::read_to_string("symlink_follow/dist/link_dir/in_dir_text.txt").unwrap(),
    );
}
