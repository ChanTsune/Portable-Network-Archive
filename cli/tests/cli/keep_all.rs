use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn archive_keep_all() {
    setup();
    TestResources::extract_in("raw/", "archive_keep_all/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_keep_all/keep_all.pna",
        "--overwrite",
        "archive_keep_all/in/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("archive_keep_all/keep_all.pna").unwrap());
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_keep_all/keep_all.pna",
        "--overwrite",
        "--out-dir",
        "archive_keep_all/out/",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--strip-components",
        "2",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("archive_keep_all/in/", "archive_keep_all/out/").unwrap();
}
