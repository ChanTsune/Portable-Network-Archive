use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn create_solid_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/solid.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap()
}
