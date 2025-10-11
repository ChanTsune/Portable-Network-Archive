use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{collections::HashSet, fs};

#[test]
fn create_archive_with_self() {
    setup();
    fs::write("self_add.pna", "").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "self_add.pna",
        "--overwrite",
        "self_add.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("self_add.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}