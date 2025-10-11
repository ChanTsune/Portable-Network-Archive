use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

#[test]
fn append_archive_with_self() {
    setup();
    // First, create an empty archive
    cli::Cli::try_parse_from(["pna", "--quiet", "c", "self_add_append.pna", "--overwrite"])
        .unwrap()
        .execute()
        .unwrap();

    // Now, try to append the archive to itself
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "self_add_append.pna",
        "self_add_append.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("self_add_append.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}