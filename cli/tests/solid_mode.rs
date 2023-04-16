use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn create_solid_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "../out/solid.pna",
        "--overwrite",
        "-r",
        "../resources/test/raw",
        "--solid",
    ]))
    .unwrap()
}
