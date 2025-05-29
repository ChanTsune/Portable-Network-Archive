#![cfg(any(unix, windows))]
use crate::utils::{archive, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

#[test]
fn create_numeric_owner() {
    setup();
    TestResources::extract_in("raw/", "numeric_owner/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "numeric_owner/numeric_owner.pna",
        "--overwrite",
        "numeric_owner/in/",
        "--keep-permission",
        "--numeric-owner",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("numeric_owner/numeric_owner.pna", |entry| {
        let p = entry.metadata().permission().unwrap();
        assert_eq!(p.uname(), "");
        assert_eq!(p.gname(), "");
    })
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "numeric_owner/numeric_owner.pna",
        "--overwrite",
        "--out-dir",
        "numeric_owner/out/",
        "--keep-permission",
        "--strip-components",
        "2",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("numeric_owner/in/", "numeric_owner/out/").unwrap();
}
