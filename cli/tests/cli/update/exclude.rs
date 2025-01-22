use super::DURATION_24_HOURS;
use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::{fs, io::prelude::*, time};

#[test]
fn archive_update_newer_mtime_with_exclude() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/"
        ),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/update_newer_mtime.pna"
        ),
        "--overwrite",
        "-r",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/"
        ),
        "--keep-timestamp",
    ]))
    .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/raw/empty.txt"
        ))
        .unwrap();
    file.write_all(b"this is updated, but this is excluded, so this should empty")
        .unwrap();
    file.set_modified(time::SystemTime::now() + DURATION_24_HOURS)
        .unwrap();

    let mut file = fs::File::options()
        .write(true)
        .truncate(true)
        .open(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/raw/text.txt"
        ))
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
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/update_newer_mtime.pna"
        ),
        "-r",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/"
        ),
        "--keep-timestamp",
        "--exclude",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/raw/empty.txt"
        ),
        "--unstable",
    ]))
    .unwrap();

    // restore original empty.txt
    TestResources::extract_in(
        "raw/empty.txt",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/"
        ),
    )
    .unwrap();

    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/update_newer_mtime.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/out/"
        ),
        "--keep-timestamp",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/"
        ))
        .to_string(),
    ]))
    .unwrap();

    diff(
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/in/"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_update_newer_mtime_with_exclude/out/"
        ),
    )
    .unwrap();
}
