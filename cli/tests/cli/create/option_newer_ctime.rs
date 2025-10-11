use crate::utils::{archive, setup};
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

fn init_resources(path: impl AsRef<std::path::Path>) {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
    fs::create_dir_all(path).unwrap();

    let keep_file = fs::File::create(path.join("keep.txt")).unwrap();
    let time = DateTime::from_str("2025-10-11T00:00:01Z")
        .unwrap()
        .to_system_time();
    let times = fs::FileTimes::new();
    #[cfg(any(windows, target_os = "macos"))]
    let times = times.set_created(time);
    #[cfg(not(any(windows, target_os = "macos")))]
    let times = times.set_modified(time);
    keep_file.set_times(times).unwrap();

    let not_keep = fs::File::create(path.join("not_keep.txt")).unwrap();
    let time = DateTime::from_str("2025-10-10T23:59:59Z")
        .unwrap()
        .to_system_time();
    let times = fs::FileTimes::new();
    #[cfg(any(windows, target_os = "macos"))]
    let times = times.set_created(time);
    #[cfg(not(any(windows, target_os = "macos")))]
    let times = times.set_modified(time);
    not_keep.set_times(times).unwrap();
}

/// Precondition: The source tree contains files whose creation times are both newer and older
///               than `2025-10-11T00:00:00Z`.
/// Action: Run `pna create` with `--newer-ctime 2025-10-11T00:00:00Z` to build an archive.
/// Expectation: The archive contains only entries whose creation times are newer than
///              `2025-10-11T00:00:00Z`.
#[test]
fn create_with_newer_ctime() {
    setup();
    init_resources("create_with_newer_ctime/in/");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_newer_ctime/create_with_newer_ctime.pna",
        "--overwrite",
        "create_with_newer_ctime/in/",
        "--keep-timestamp",
        "--newer-ctime",
        "2025-10-11T00:00:00Z",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_with_newer_ctime/create_with_newer_ctime.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in ["create_with_newer_ctime/in/keep.txt"] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
