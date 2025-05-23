mod exclude;

use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::{fs, io::prelude::*, time};

const DURATION_24_HOURS: time::Duration = time::Duration::from_secs(24 * 60 * 60);

#[test]
fn archive_update_newer_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_update_newer_mtime/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_newer_mtime/update_newer_mtime.pna",
        "--overwrite",
        "archive_update_newer_mtime/in/",
        "--keep-timestamp",
    ]))
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_newer_mtime/in/raw/empty.txt")
        .unwrap();
    file.write_all(b"this is updated, but mtime older than now, so this should empty")
        .unwrap();
    file.set_modified(time::SystemTime::now() - DURATION_24_HOURS)
        .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_newer_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--newer-mtime",
        "archive_update_newer_mtime/update_newer_mtime.pna",
        "archive_update_newer_mtime/in/",
        "--keep-timestamp",
    ]))
    .unwrap();

    // restore original empty.txt
    TestResources::extract_in("raw/empty.txt", "archive_update_newer_mtime/in/").unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_update_newer_mtime/update_newer_mtime.pna",
        "--overwrite",
        "--out-dir",
        "archive_update_newer_mtime/out/",
        "--keep-timestamp",
        "--strip-components",
        &components_count("archive_update_newer_mtime/in/").to_string(),
    ]))
    .unwrap();

    diff(
        "archive_update_newer_mtime/in/",
        "archive_update_newer_mtime/out/",
    )
    .unwrap();
}

#[test]
fn archive_update_older_mtime() {
    setup();
    TestResources::extract_in("raw/", "archive_update_older_mtime/in/").unwrap();
    TestResources::extract_in("raw/", "archive_update_older_mtime/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_older_mtime/update_older_mtime.pna",
        "--overwrite",
        "archive_update_older_mtime/in/",
        "--keep-timestamp",
    ]))
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_older_mtime/in/raw/empty.txt")
        .unwrap();
    file.write_all(b"this is updated, but mtime newer than now, so this should empty")
        .unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open("archive_update_older_mtime/in/raw/text.txt")
        .unwrap();
    file.write_all(b"updated!").unwrap();
    file.set_modified(time::SystemTime::now() - DURATION_24_HOURS)
        .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--older-mtime",
        "archive_update_older_mtime/update_older_mtime.pna",
        "archive_update_older_mtime/in/",
        "--keep-timestamp",
    ]))
    .unwrap();

    // restore original empty.txt
    TestResources::extract_in("raw/empty.txt", "archive_update_older_mtime/in/").unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_update_older_mtime/update_older_mtime.pna",
        "--overwrite",
        "--out-dir",
        "archive_update_older_mtime/out/",
        "--keep-timestamp",
        "--strip-components",
        &components_count("archive_update_older_mtime/in/").to_string(),
    ]))
    .unwrap();

    diff(
        "archive_update_older_mtime/in/",
        "archive_update_older_mtime/out/",
    )
    .unwrap();
}

#[test]
fn archive_update_deletion() {
    setup();
    TestResources::extract_in("raw/", "archive_update_deletion/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "archive_update_deletion/update_deletion.pna",
        "--overwrite",
        "archive_update_deletion/in/",
        "--keep-timestamp",
    ]))
    .unwrap();

    fs::remove_file("archive_update_deletion/in/raw/empty.txt").unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--newer-mtime",
        "archive_update_deletion/update_deletion.pna",
        "archive_update_deletion/in/",
        "--keep-timestamp",
    ]))
    .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "archive_update_deletion/update_deletion.pna",
        "--overwrite",
        "--out-dir",
        "archive_update_deletion/out/",
        "--keep-timestamp",
        "--strip-components",
        &components_count("archive_update_deletion/in/").to_string(),
    ]))
    .unwrap();

    // restore original empty.txt
    TestResources::extract_in("raw/empty.txt", "archive_update_deletion/in/").unwrap();

    diff(
        "archive_update_deletion/in/",
        "archive_update_deletion/out/",
    )
    .unwrap();
}
