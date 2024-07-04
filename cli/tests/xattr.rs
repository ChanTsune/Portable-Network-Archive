use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_xattr() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/manipulate_xattr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "xattr",
        "set",
        &format!("{}/manipulate_xattr.pna", env!("CARGO_TARGET_TMPDIR")),
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
        &format!("{}/manipulate_xattr.pna", env!("CARGO_TARGET_TMPDIR")),
        "resources/test/raw/empty.txt",
        "--name",
        "user.name",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/manipulate_xattr.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/manipulate_xattr/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-xattr",
    ]))
    .unwrap();
}
