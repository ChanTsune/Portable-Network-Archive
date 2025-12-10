use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: A solid archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `--unsolid` and `-x` to remove execute.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-) and archive is unsolidified.
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
        "-f",
        "chmod_unsolid/chmod_unsolid.pna",
        "--",
        "-x",
        "chmod_unsolid/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_unsolid/chmod_unsolid.pna", |entry| {
        if entry.header().path() == "chmod_unsolid/in/raw/text.txt" {
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
