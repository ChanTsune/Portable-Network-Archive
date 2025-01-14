use crate::utils::{components_count, copy_dir_all, diff::diff, setup};
use clap::Parser;
use portable_network_archive::{cli, command};
use std::fs;

#[test]
fn archive_keep_all() {
    setup();
    copy_dir_all(
        "../resources/test/raw",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_keep_all/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_keep_all/keep_all.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_keep_all/in/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    assert!(fs::exists(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/archive_keep_all/keep_all.pna"
    ))
    .unwrap());
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_keep_all/keep_all.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_keep_all/out/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--strip-components",
        &components_count(concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/archive_keep_all/in/"
        ))
        .to_string(),
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_keep_all/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/archive_keep_all/out/"),
    )
    .unwrap();
}
