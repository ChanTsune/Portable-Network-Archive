use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

#[test]
fn chmod_keep_solid() {
    setup();
    TestResources::extract_in("raw/", "chmod_keep_solid/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_keep_solid/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_keep_solid/chmod_keep_solid.pna",
        "--overwrite",
        "--solid",
        "chmod_keep_solid/in/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "--keep-solid",
        "chmod_keep_solid/chmod_keep_solid.pna",
        "--",
        "-x",
        "chmod_keep_solid/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod_keep_solid/chmod_keep_solid.pna",
        "--overwrite",
        "--out-dir",
        "chmod_keep_solid/out/",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    #[cfg(unix)]
    {
        let meta = fs::symlink_metadata("chmod_keep_solid/out/raw/text.txt").unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o666);
    }

    diff("chmod_keep_solid/in/", "chmod_keep_solid/out/").unwrap();
}
