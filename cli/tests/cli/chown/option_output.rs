use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: An archive contains entries with permission metadata.
/// Action: Run `pna experimental chown` with `--output` to a new path.
/// Expectation: The output archive has the updated owner; the original is untouched.
#[test]
fn chown_output() {
    setup();

    archive::create_archive_with_permissions(
        "chown_output.pna",
        &[FileEntryDef {
            path: "target.txt",
            content: b"target",
            permission: 0o644,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        "-f",
        "chown_output.pna",
        "--output",
        "chown_output_out.pna",
        "new_user",
        "target.txt",
        "--no-owner-lookup",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let uname_of = |path: &str| {
        let mut uname = String::new();
        archive::for_each_entry(path, |entry| {
            uname = entry.metadata().owner_user_name().unwrap().to_string();
        })
        .unwrap();
        uname
    };

    assert_eq!(uname_of("chown_output.pna"), "user");
    assert_eq!(uname_of("chown_output_out.pna"), "new_user");
}
