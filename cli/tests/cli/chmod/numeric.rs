use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

#[test]
fn chmod_numeric_mode() {
    setup();
    TestResources::extract_in("raw/", "chmod_numeric/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_numeric/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_numeric/chmod_numeric.pna",
        "--overwrite",
        "chmod_numeric/in/",
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
        "chmod_numeric/chmod_numeric.pna",
        "644",
        "chmod_numeric/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod_numeric/chmod_numeric.pna",
        "--overwrite",
        "--out-dir",
        "chmod_numeric/out/",
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
        let meta = fs::symlink_metadata("chmod_numeric/out/raw/text.txt").unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o644);
    }

    diff("chmod_numeric/in/", "chmod_numeric/out/").unwrap();
}
