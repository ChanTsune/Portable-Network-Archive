use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

#[test]
fn chmod_unsolid() {
    setup();
    TestResources::extract_in("raw/", "chmod_unsolid/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_unsolid/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_unsolid/chmod_unsolid.pna",
        "--overwrite",
        "--solid",
        "chmod_unsolid/in/",
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
        "--unsolid",
        "chmod_unsolid/chmod_unsolid.pna",
        "--",
        "-x",
        "chmod_unsolid/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod_unsolid/chmod_unsolid.pna",
        "--overwrite",
        "--out-dir",
        "chmod_unsolid/out/",
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
        let meta = fs::symlink_metadata("chmod_unsolid/out/raw/text.txt").unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o666);
    }

    diff("chmod_unsolid/in/", "chmod_unsolid/out/").unwrap();
}
