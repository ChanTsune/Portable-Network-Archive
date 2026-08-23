use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive holds several entries carrying permission metadata.
/// Action: Run `pna strip` naming only some of the entries.
/// Expectation: The named entries lose their metadata; the others keep it and every entry survives.
#[test]
fn strip_only_named_entries() {
    setup();
    let path = "strip_file_operands.pna";
    archive::create_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "keep.txt",
                content: b"keep",
                permission: 0o644,
            },
            FileEntryDef {
                path: "strip.txt",
                content: b"strip",
                permission: 0o644,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "strip",
        "--overwrite",
        "-f",
        path,
        "strip.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entries = Vec::new();
    archive::for_each_entry(path, |entry| {
        entries.push((
            entry.header().path().to_string(),
            entry.metadata().permission_mode().map(|m| m.get()),
        ));
    })
    .unwrap();
    assert_eq!(
        entries,
        [
            ("keep.txt".to_string(), Some(0o644)),
            ("strip.txt".to_string(), None),
        ]
    );
}
