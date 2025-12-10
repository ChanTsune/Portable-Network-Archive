use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: A solid archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `--unsolid` and `-x` to remove execute.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-) and archive is unsolidified.
#[test]
fn chmod_unsolid() {
    setup();

    archive::create_solid_archive_with_permissions(
        "chmod_unsolid.pna",
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
        "--unsolid",
        "-f",
        "chmod_unsolid.pna",
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_unsolid.pna", |entry| {
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
