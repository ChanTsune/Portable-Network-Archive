use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive contains a file with permission 0o777.
/// Action: Run `pna experimental chmod` with `--output` to a new path.
/// Expectation: The output archive has the updated mode; the original is untouched.
#[test]
fn chmod_output() {
    setup();

    archive::create_archive_with_permissions(
        "chmod_output.pna",
        &[FileEntryDef {
            path: "test.txt",
            content: b"test content",
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
        "chmod_output.pna",
        "--output",
        "chmod_output_out.pna",
        "--",
        "-x",
        "test.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::modes_by_entry("chmod_output.pna"),
        vec![("test.txt".to_string(), Some(0o777))]
    );
    assert_eq!(
        archive::modes_by_entry("chmod_output_out.pna"),
        vec![("test.txt".to_string(), Some(0o666))]
    );
}
