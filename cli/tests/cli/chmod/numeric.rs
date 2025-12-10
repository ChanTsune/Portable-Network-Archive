use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::prelude::*;

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with numeric mode `644`.
/// Expectation: The archive entry's permission becomes 0o644 (rw-r--r--).
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
        "-f",
        "chmod_numeric/chmod_numeric.pna",
        "644",
        "chmod_numeric/in/raw/text.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_numeric/chmod_numeric.pna", |entry| {
        if entry.header().path() == "chmod_numeric/in/raw/text.txt" {
            let perm = entry
                .metadata()
                .permission()
                .expect("entry should have permission metadata");
            assert_eq!(
                perm.permissions() & 0o777,
                0o644,
                "644 on 0o777 should yield 0o644"
            );
        }
    })
    .unwrap();
}
