use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An encrypted archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with password and `-x` to remove execute.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-).
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
        "-f",
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

    archive::for_each_entry_with_password(
        "chmod_password/chmod_password.pna",
        Some("password"),
        |entry| {
            if entry.header().path() == "chmod_password/in/raw/text.txt" {
                let perm = entry
                    .metadata()
                    .permission()
                    .expect("entry should have permission metadata");
                assert_eq!(
                    perm.permissions() & 0o777,
                    0o666,
                    "-x on 0o777 should yield 0o666"
                );
            }
        },
    )
    .unwrap();
}
