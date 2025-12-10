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

use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `-x` to remove execute permission for all.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-).
#[test]
fn archive_chmod() {
    setup();

    archive::create_archive_with_permissions(
        "chmod.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "-f",
        "chmod.pna",
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
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
