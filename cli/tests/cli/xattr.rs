use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_xattr_set() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/xattr_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        &format!("{}/xattr_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "resources/test/raw/empty.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        &format!("{}/xattr_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/empty.txt",
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/xattr_set.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/xattr_set/", env!("CARGO_TARGET_TMPDIR")),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ]))
    .unwrap();
}

#[test]
fn archive_xattr_remove() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/xattr_remove.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
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
        &format!("{}/xattr_remove.pna", env!("CARGO_TARGET_TMPDIR")),
        "--name",
        "user.name",
        "--value",
        "pna developers!",
        "resources/test/raw/empty.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        &format!("{}/xattr_remove.pna", env!("CARGO_TARGET_TMPDIR")),
        "--remove",
        "user.name",
        "resources/test/raw/empty.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "get",
        &format!("{}/xattr_remove.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/empty.txt",
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/xattr_remove.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/xattr_remove/", env!("CARGO_TARGET_TMPDIR")),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
    ]))
    .unwrap();
}
