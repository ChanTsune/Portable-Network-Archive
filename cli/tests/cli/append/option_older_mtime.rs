use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli::{self, value::DateTime};
use std::collections::HashSet;
use std::fs;
use std::str::FromStr;

fn init_archive(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();
    TestResources::extract_in("empty.pna", path).unwrap();
    fs::rename(
        path.join("empty.pna"),
        path.join("append_with_older_mtime.pna"),
    )
    .unwrap();
}

fn init_sources(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();

    let keep_file = fs::File::create(path.join("keep.txt")).unwrap();
    let keep_time = fs::FileTimes::new().set_modified(
        DateTime::from_str("2025-10-10T23:59:59Z")
            .unwrap()
            .to_system_time(),
    );
    keep_file.set_times(keep_time).unwrap();

    let skip_file = fs::File::create(path.join("skip.txt")).unwrap();
    let skip_time = fs::FileTimes::new().set_modified(
        DateTime::from_str("2025-10-11T00:00:01Z")
            .unwrap()
            .to_system_time(),
    );
    skip_file.set_times(skip_time).unwrap();
}

/// Precondition: The append target is an empty archive, and the source tree has files whose
///               modification times are both newer and older than `2025-10-11T00:00:00Z`.
/// Action: Run `pna append` with `--older-mtime 2025-10-11T00:00:00Z` to append into the archive.
/// Expectation: The archive contains only entries whose modification times are older than
///              `2025-10-11T00:00:00Z`.
#[test]
fn append_with_older_mtime() {
    setup();
    init_archive("append_with_older_mtime/");
    init_sources("append_with_older_mtime/in/");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--keep-timestamp",
        "--older-mtime",
        "2025-10-11T00:00:00Z",
        "append_with_older_mtime/append_with_older_mtime.pna",
        "append_with_older_mtime/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "append_with_older_mtime/append_with_older_mtime.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    let required_entries = ["append_with_older_mtime/in/keep.txt"];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
