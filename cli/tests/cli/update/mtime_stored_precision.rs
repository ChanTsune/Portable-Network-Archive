use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    fs,
    time::{Duration, SystemTime},
};

fn archived_mtimes(path: &str) -> Vec<Option<pna::Duration>> {
    let mut mtimes = Vec::new();
    archive::for_each_entry(path, |entry| mtimes.push(entry.metadata().modified())).unwrap();
    mtimes
}

/// Precondition: Archive stores a file mtime with zero sub-second precision.
/// Action: Filesystem mtime moves to the same whole second with nonzero nanoseconds, then update runs.
/// Expectation: The entry is left untouched and no second entry is appended.
/// Returns early on filesystems without nanosecond timestamp support.
#[test]
fn update_keeps_entry_when_only_subsecond_differs() {
    setup();
    let dir = "update_mtime_keeps_entry";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();
    let whole_second = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    filetime::set_file_mtime(
        &file_path,
        filetime::FileTime::from_system_time(whole_second),
    )
    .unwrap();

    let archive_path = format!("{dir}/archive.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive_path,
        "--overwrite",
        "--keep-timestamp",
        &file_path,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let with_nanos = whole_second + Duration::from_nanos(123_456_700);
    filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(with_nanos)).unwrap();
    if fs::metadata(&file_path).unwrap().modified().unwrap() != with_nanos {
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        &archive_path,
        "--keep-timestamp",
        &file_path,
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archived_mtimes(&archive_path),
        [Some(pna::Duration::seconds(86400))],
    );
}
