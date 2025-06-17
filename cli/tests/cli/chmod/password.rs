use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

#[test]
fn chmod_with_password() {
    setup();
    TestResources::extract_in("raw/", "chmod_password/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_password/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_password/chmod_password.pna",
        "--overwrite",
        "chmod_password/in/",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
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
        "chmod_password/chmod_password.pna",
        "--password",
        "password",
        "--",
        "-x",
        "chmod_password/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod_password/chmod_password.pna",
        "--overwrite",
        "--out-dir",
        "chmod_password/out/",
        "--password",
        "password",
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
        let meta = fs::symlink_metadata("chmod_password/out/raw/text.txt").unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o666);
    }

    diff("chmod_password/in/", "chmod_password/out/").unwrap();
}
