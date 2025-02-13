use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

#[test]
fn archive_chmod() {
    setup();
    TestResources::extract_in("raw/", "chmod/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions("chmod/in/raw/text.txt", fs::Permissions::from_mode(0o777)).unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod/chmod.pna",
        "--overwrite",
        "chmod/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "chmod/chmod.pna",
        "--",
        "-x",
        "chmod/in/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod/chmod.pna",
        "--overwrite",
        "--out-dir",
        "chmod/out/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
        "--strip-components",
        "2",
    ]))
    .unwrap();
    #[cfg(unix)]
    {
        let meta = fs::symlink_metadata("chmod/out/raw/text.txt").unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o666);
    }

    diff("chmod/in/", "chmod/out/").unwrap();
}
