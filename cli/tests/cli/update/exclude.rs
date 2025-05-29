use super::DURATION_24_HOURS;
use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io::prelude::*, time};

#[test]
fn archive_update_newer_mtime_with_exclude() {
    setup();
    TestResources::extract_in("raw/", "archive_update_newer_mtime_with_exclude/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_newer_mtime_with_exclude/update_newer_mtime.pna",
        "--overwrite",
        "archive_update_newer_mtime_with_exclude/in/",
        "--keep-timestamp",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_newer_mtime_with_exclude/in/raw/empty.txt")
        .unwrap();
    file.write_all(b"this is updated, but this is excluded, so this should empty")
        .unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_newer_mtime_with_exclude/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--newer-mtime",
        "archive_update_newer_mtime_with_exclude/update_newer_mtime.pna",
        "archive_update_newer_mtime_with_exclude/in/",
        "--keep-timestamp",
        "--exclude",
        "archive_update_newer_mtime_with_exclude/in/raw/empty.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // restore original empty.txt
    TestResources::extract_in(
        "raw/empty.txt",
        "archive_update_newer_mtime_with_exclude/in/",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_update_newer_mtime_with_exclude/update_newer_mtime.pna",
        "--overwrite",
        "--out-dir",
        "archive_update_newer_mtime_with_exclude/out/",
        "--keep-timestamp",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff(
        "archive_update_newer_mtime_with_exclude/in/",
        "archive_update_newer_mtime_with_exclude/out/",
    )
    .unwrap();
}
