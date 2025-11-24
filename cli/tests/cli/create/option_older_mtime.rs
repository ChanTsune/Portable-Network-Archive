use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{
    cli::{self, value::DateTime},
    command::Command,
};
use std::collections::HashSet;
use std::fs;
use std::str::FromStr;

fn init_resources(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();

    let keep_file = fs::File::create(path.join("keep.txt")).unwrap();
    let time = fs::FileTimes::new().set_modified(
        DateTime::from_str("2025-10-10T23:59:59Z")
            .unwrap()
            .to_system_time(),
    );
    keep_file.set_times(time).unwrap();

    let not_keep = fs::File::create(path.join("not_keep.txt")).unwrap();
    let time = fs::FileTimes::new().set_modified(
        DateTime::from_str("2025-10-11T00:00:01Z")
            .unwrap()
            .to_system_time(),
    );
    not_keep.set_times(time).unwrap();
}

/// Precondition: The source tree contains files whose modification times are both newer and older
///               than `2025-10-11T00:00:00Z`.
/// Action: Run `pna create` with `--older-mtime 2025-10-11T00:00:00Z` to build an archive.
/// Expectation: The archive contains only entries whose modification times are older than
///              `2025-10-11T00:00:00Z`.
#[test]
fn create_with_older_mtime() {
    setup();
    init_resources("create_with_older_mtime/in/");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_older_mtime/create_with_older_mtime.pna",
        "--overwrite",
        "create_with_older_mtime/in/",
        "--keep-timestamp",
        "--older-mtime",
        "2025-10-11T00:00:00Z",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_with_older_mtime/create_with_older_mtime.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    let required_entries = ["create_with_older_mtime/in/keep.txt"];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
