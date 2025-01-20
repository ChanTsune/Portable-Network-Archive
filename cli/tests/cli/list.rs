use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_list() {
    setup();
    TestResources::extract_in("raw/", concat!(env!("CARGO_TARGET_TMPDIR"), "/list/in/")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list/list.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list/in/"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list/list.pna"),
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid/list_solid.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid/in/"),
        "--solid",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid/list_solid.pna"),
        "--solid",
    ]))
    .unwrap();
}

#[test]
fn archive_list_detail() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_detail/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_detail/list_detail.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_detail/in/"),
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_detail/list_detail.pna"),
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid_detail() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid_detail/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/list_solid_detail/list_solid_detail.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid_detail/in/"),
        "--solid",
        #[cfg(not(target_os = "netbsd"))]
        "--keep-xattr",
        "--keep-timestamp",
        "--keep-permission",
        "--password",
        "password",
        "--aes",
        "ctr",
        "--argon2",
        "t=1,m=50",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/list_solid_detail/list_solid_detail.pna"
        ),
        "--solid",
        "--password",
        "password",
    ]))
    .unwrap();
}

#[test]
fn archive_list_jsonl() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_jsonl/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_jsonl/list_jsonl.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_jsonl/in/"),
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
        "--argon2",
        "t=1,m=50",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "jsonl",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_jsonl/list_jsonl.pna"),
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid_jsonl() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid_jsonl/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/list_solid_jsonl/list_solid_jsonl.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid_jsonl/in/"),
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
        "--argon2",
        "t=1,m=50",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "jsonl",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/list_solid_jsonl/list_solid_jsonl.pna"
        ),
        "--solid",
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn archive_list_tree() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_tree/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_tree/list_tree.pna"),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_tree/in/"),
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
        "--argon2",
        "t=1,m=50",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "tree",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_tree/list_tree.pna"),
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}

#[test]
fn archive_list_solid_tree() {
    setup();
    TestResources::extract_in(
        "raw/",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid_tree/in/"),
    )
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/list_solid_tree/list_solid_tree.pna"
        ),
        "--overwrite",
        "-r",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/list_solid_tree/in/"),
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
        "--argon2",
        "t=1,m=50",
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "list",
        "-l",
        "--format",
        "tree",
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/list_solid_tree/list_solid_tree.pna"
        ),
        "--solid",
        "--password",
        "password",
        "--unstable",
    ]))
    .unwrap();
}
