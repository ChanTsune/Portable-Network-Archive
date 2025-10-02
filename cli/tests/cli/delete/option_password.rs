use crate::utils::{archive, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` with `--password` to build an encrypted archive, then delete entries
///          by `pna experimental delete` with `--password`
/// Expectation: Only the entries specified is removed from the encrypted archive; all
///         other entries remain.
#[test]
fn delete_with_password() {
    setup();
    TestResources::extract_in("raw/", "delete_password/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_password/delete_password.pna",
        "--overwrite",
        "delete_password/in/",
        "--password",
        "password",
        "--aes",
        "ctr",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_password/delete_password.pna",
        "**/raw/empty.txt",
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry_with_password(
        "delete_password/delete_password.pna",
        "password",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "delete_password/in/raw/images/icon.png",
        "delete_password/in/raw/images/icon.svg",
        "delete_password/in/raw/text.txt",
        "delete_password/in/raw/pna/nest.pna",
        "delete_password/in/raw/parent/child.txt",
        "delete_password/in/raw/pna/empty.pna",
        "delete_password/in/raw/first/second/third/pna.txt",
        "delete_password/in/raw/images/icon.bmp",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}, {seen:?}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
