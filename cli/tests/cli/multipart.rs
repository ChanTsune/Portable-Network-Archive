use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;

#[test]
fn multipart_archive() {
    setup();
    TestResources::extract_in("multipart_test.txt", "./multipart_archive/in/").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "./multipart_archive/multipart.pna",
        "--overwrite",
        "./multipart_archive/in/multipart_test.txt",
        "--unstable",
        "--split",
        "110",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "./multipart_archive/multipart.part1.pna",
        "--overwrite",
        "--out-dir",
        "./multipart_archive/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("./multipart_archive/in/", "./multipart_archive/out/").unwrap();
}
