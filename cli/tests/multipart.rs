use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn create_multipart_archive() {
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        &format!("{}/multipart.pna", env!("CARGO_TARGET_TMPDIR")),
        "--overwrite",
        "../resources/test/multipart_test.txt",
        "--unstable",
        "--split",
        "110",
    ]))
    .unwrap()
}
