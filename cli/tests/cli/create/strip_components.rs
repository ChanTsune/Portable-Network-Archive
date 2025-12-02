use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains `a/b/file.txt` under the input directory.
/// Action: Run `pna create` with `--strip-components 1`.
/// Expectation: The archived entry is stored with the first component removed.
#[test]
fn create_command_strips_components_on_store() {
    setup();

    fs::create_dir_all("create_strip_components/in/a/b").unwrap();
    fs::write("create_strip_components/in/a/b/file.txt", b"payload").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "--strip-components",
        "1",
        "--unstable",
        "--overwrite",
        "-f",
        "create_strip_components/archive.pna",
        "create_strip_components/in/a/b/file.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_strip_components/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    let required_entries = ["in/a/b/file.txt"];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
