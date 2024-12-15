use crate::utils::setup;
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn archive_chmod() {
    setup();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/chmod.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--keep-permission",
        #[cfg(windows)]
        {
            "--unstable"
        },
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        &format!("{}/chmod.pna", env!("CARGO_TARGET_TMPDIR")),
        "--",
        "-w",
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chown",
        &format!("{}/chmod.pna", env!("CARGO_TARGET_TMPDIR")),
        "+w",
        "resources/test/raw/text.txt",
    ]))
    .unwrap();
}
