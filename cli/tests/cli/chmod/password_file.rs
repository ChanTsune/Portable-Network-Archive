use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

#[test]
fn chmod_with_password_file() {
    setup();
    TestResources::extract_in("raw/", "chmod_password_file/in/").unwrap();
    let password_file_path = "chmod_password_file/password_file";
    let password = "chmod_password_file";
    fs::write(password_file_path, password).unwrap();

    #[cfg(unix)]
    fs::set_permissions(
        "chmod_password_file/in/raw/text.txt",
        fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod_password_file/chmod_password_file.pna",
        "--overwrite",
        "chmod_password_file/in/",
        "--keep-permission",
        "--password",
        password,
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
        "chmod_password_file/chmod_password_file.pna",
        "--password-file",
        password_file_path,
        "--",
        "-x",
        "chmod_password_file/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "chmod_password_file/chmod_password_file.pna",
        "--overwrite",
        "--out-dir",
        "chmod_password_file/out/",
        "--password-file",
        password_file_path,
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
        let meta = fs::symlink_metadata("chmod_password_file/out/raw/text.txt").unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o666);
    }

    diff("chmod_password_file/in/", "chmod_password_file/out/").unwrap();
}
