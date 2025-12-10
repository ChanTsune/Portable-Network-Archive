mod edge_cases;
mod glob_pattern;
mod keep_solid;
mod missing_file;
mod multiple_clauses;
mod numeric;
mod password;
mod password_file;
mod symbolic_all;
mod symbolic_combined;
mod symbolic_group;
mod symbolic_other;
mod symbolic_user;
mod unsolid;

use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `-x` to remove execute permission for all.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-).
#[test]
fn archive_chmod() {
    setup();
    TestResources::extract_in("raw/", "chmod/in/").unwrap();

    #[cfg(unix)]
    fs::set_permissions("chmod/in/raw/text.txt", fs::Permissions::from_mode(0o777)).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "chmod/chmod.pna",
        "--overwrite",
        "chmod/in/",
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
        "-f",
        "chmod/chmod.pna",
        "--",
        "-x",
        "chmod/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod/chmod.pna", |entry| {
        if entry.header().path() == "chmod/in/raw/text.txt" {
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
    })
    .unwrap();
}
