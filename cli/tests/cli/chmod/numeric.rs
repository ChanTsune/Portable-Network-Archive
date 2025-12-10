use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: An archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with numeric mode `644`.
/// Expectation: The archive entry's permission becomes 0o644 (rw-r--r--).
#[test]
fn chmod_numeric_mode() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_numeric.pna",
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
        "chmod_numeric.pna",
        "644",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry("chmod_numeric.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
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
