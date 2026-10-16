use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;

/// Precondition: A solid archive holds several entries carrying permission metadata.
/// Action: Run `pna strip --unsolid` naming only some of the entries.
/// Expectation: Only the named entries lose their metadata and every entry becomes a normal entry.
#[test]
fn strip_unsolid_only_named_entries() {
    setup();
    let path = "strip_unsolid.pna";
    archive::create_solid_archive_with_permissions(
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
        "--unsolid",
        "-f",
        path,
        "strip.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut archive = pna::Archive::open(path).unwrap();
    let layout = archive
        .entries()
        .map(|entry| matches!(entry.unwrap(), pna::ReadEntry::Solid(_)))
        .collect::<Vec<_>>();
    assert_eq!(layout, [false, false]);

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
