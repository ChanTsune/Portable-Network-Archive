use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;

/// Precondition: The source tree contains both files and directories.
/// Action: Run `pna create` to build an archive, then delete entries by
///         `pna experimental delete` with `--include`.
/// Expectation: Only the entries specified by `--include` are removed; all other files remain.
#[test]
fn delete_with_include() {
    setup();
    TestResources::extract_in("raw/", "delete_with_include/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_with_include/include.pna",
        "--overwrite",
        "delete_with_include/in/",
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
        "delete_with_include/include.pna",
        "*",
        "--include",
        "**/raw/text.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("delete_with_include/include.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    for required in [
        "delete_with_include/in/raw/pna/empty.pna",
        "delete_with_include/in/raw/images/icon.svg",
        "delete_with_include/in/raw/pna/nest.pna",
        "delete_with_include/in/raw/images/icon.png",
        "delete_with_include/in/raw/first/second/third/pna.txt",
        "delete_with_include/in/raw/empty.txt",
        "delete_with_include/in/raw/images/icon.bmp",
        "delete_with_include/in/raw/parent/child.txt",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}, {seen:?}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
