use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

#[test]
fn create_with_substitution() {
    setup();
    TestResources::extract_in("raw/", "create_with_substitution/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_substitution/create_with_substitution.pna",
        "--overwrite",
        "create_with_substitution/in/",
        "-s",
        "#create_with_substitution/in/##",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::exists("create_with_substitution/create_with_substitution.pna").unwrap());

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_with_substitution/create_with_substitution.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "raw",
        "raw/empty.txt",
        "raw/text.txt",
        "raw/first",
        "raw/first/second",
        "raw/first/second/third",
        "raw/first/second/third/pna.txt",
        "raw/parent",
        "raw/parent/child.txt",
        "raw/images",
        "raw/images/icon.bmp",
        "raw/images/icon.png",
        "raw/images/icon.svg",
        "raw/pna",
        "raw/pna/empty.pna",
        "raw/pna/nest.pna",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
