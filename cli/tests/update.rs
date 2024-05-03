use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_update_newer_mtime() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/update_all.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
    ]))
    .unwrap();

    let file = std::fs::File::open("../resources/test/raw/empty.txt").unwrap();
    file.set_modified(std::time::SystemTime::now()).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--newer-mtime",
        &format!("{}/update_all.pna", env!("CARGO_TARGET_TMPDIR")),
        "-r",
        "../resources/test/raw",
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        &format!("{}/update_all.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "--out-dir",
        &format!("{}/update_all/", env!("CARGO_TARGET_TMPDIR")),
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
    ]))
    .unwrap();
}
