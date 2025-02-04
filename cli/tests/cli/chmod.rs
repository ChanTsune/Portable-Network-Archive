use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_chmod() {
    setup();
    TestResources::extract_in("raw/", concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/in/")).unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/chmod.pna"),
        "--overwrite",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/in/"),
        "--keep-permission",
        #[cfg(windows)]
        "--unstable",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/chmod.pna"),
        "--",
        "-w",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/in/raw/text.txt"),
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/chmod.pna"),
        "+w",
        concat!(env!("CARGO_TARGET_TMPDIR"), "/chmod/in/raw/text.txt"),
    ]))
    .unwrap();
}
