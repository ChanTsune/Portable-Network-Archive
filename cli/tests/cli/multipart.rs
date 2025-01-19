use crate::utils::{diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command};

#[test]
fn multipart_archive() {
    setup();
    TestResources::extract_in("multipart_test.txt", "./multipart_archive/in/").unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "c",
        "./multipart_archive/multipart.pna",
        "--overwrite",
        "./multipart_archive/in/multipart_test.txt",
        "--unstable",
        "--split",
        "110",
    ]))
    .unwrap();
    command::entry(cli::Cli::parse_from([
        "pna",
        "--quiet",
        "x",
        "./multipart_archive/multipart.part1.pna",
        "--overwrite",
        "--out-dir",
        "./multipart_archive/out/",
        "--strip-components",
        "2",
    ]))
    .unwrap();

    diff("./multipart_archive/in/", "./multipart_archive/out/").unwrap();
}
