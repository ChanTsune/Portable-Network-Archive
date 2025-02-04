use crate::utils::{components_count, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_xattr_set() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/xattr_set.pna"),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/in/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/xattr_set.pna"),
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/in/raw/empty.txt"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/xattr_set.pna"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/in/raw/empty.txt"),
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/xattr_set.pna"),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/out/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_set/out/"),
    )
    .unwrap();
}

#[test]
fn archive_xattr_remove() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_remove/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/xattr_remove.pna"
        ),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_remove/in/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/xattr_remove.pna"
        ),
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/in/raw/empty.txt"
        ),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/xattr_remove.pna"
        ),
        "--remove",
        "user.name",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/in/raw/empty.txt"
        ),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/xattr_remove.pna"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/in/raw/empty.txt"
        ),
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/xattr_remove/xattr_remove.pna"
        ),
        "--overwrite",
        "--out-dir",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_remove/out/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--strip-components",
        &components_count(concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_remove/in/")).to_string(),
    ]))
    .unwrap();

    diff(
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_remove/in/"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/xattr_remove/out/"),
    )
    .unwrap();
}
