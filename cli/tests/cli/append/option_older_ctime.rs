use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::{
    cli::{self, value::DateTime},
    command::Command,
};
use std::collections::HashSet;
use std::fs;
#[cfg(target_os = "macos")]
use std::os::macos::fs::FileTimesExt;
#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;
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
        path.join("append_with_older_ctime.pna"),
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
    let time = DateTime::from_str("2025-10-10T23:59:59Z")
        .unwrap()
        .to_system_time();
    let times = fs::FileTimes::new();
    #[cfg(any(windows, target_os = "macos"))]
    let times = times.set_created(time);
    #[cfg(not(any(windows, target_os = "macos")))]
    let times = times.set_modified(time);
    keep_file.set_times(times).unwrap();

    let skip_file = fs::File::create(path.join("skip.txt")).unwrap();
    let time = DateTime::from_str("2025-10-11T00:00:01Z")
        .unwrap()
        .to_system_time();
    let times = fs::FileTimes::new();
    #[cfg(any(windows, target_os = "macos"))]
    let times = times.set_created(time);
    #[cfg(not(any(windows, target_os = "macos")))]
    let times = times.set_modified(time);
    skip_file.set_times(times).unwrap();
}

/// Precondition: The append target is an empty archive, and the source tree has files whose
///               creation times are both newer and older than `2025-10-11T00:00:00Z`.
/// Action: Run `pna append` with `--older-ctime 2025-10-11T00:00:00Z` to append into the archive.
/// Expectation: The archive contains only entries whose creation times are older than
///              `2025-10-11T00:00:00Z`.
#[test]
fn append_with_older_ctime() {
    setup();
    init_archive("append_with_older_ctime/");
    init_sources("append_with_older_ctime/in/");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "append",
        "--keep-timestamp",
        "--older-ctime",
        "2025-10-11T00:00:00Z",
        "append_with_older_ctime/append_with_older_ctime.pna",
        "append_with_older_ctime/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "append_with_older_ctime/append_with_older_ctime.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in ["append_with_older_ctime/in/keep.txt"] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
