use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_list() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        &format!("{}/list.pna", env!("CARGO_TARGET_TMPDIR")),
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        &format!("{}/list_solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
    ]))
    .unwrap();
}

#[test]
fn archive_list_detail() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        &format!("{}/list_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid_detail() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_solid_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        &format!("{}/list_solid_detail.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn archive_list_jsonl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_jsonl.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(feature = "acl")]
        "--keep-acl",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "jsonl",
        &format!("{}/list_jsonl.pna", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid_jsonl() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_solid_jsonl.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(feature = "acl")]
        "--keep-acl",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "jsonl",
        &format!("{}/list_solid_jsonl.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn archive_list_tree() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_tree.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(feature = "acl")]
        "--keep-acl",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "tree",
        &format!("{}/list_tree.pna", env!("CARGO_TARGET_TMPDIR")),
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid_tree() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/list_solid_tree.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        #[cfg(feature = "acl")]
        "--keep-acl",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "tree",
        &format!("{}/list_solid_tree.pna", env!("CARGO_TARGET_TMPDIR")),
        "--solid",
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}
