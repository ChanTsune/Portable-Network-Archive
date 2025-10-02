use crate::utils::{setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_accepts_legacy_compression_flags() {
    setup();
    TestResources::extract_in("raw/text.txt", "compression/input/").unwrap();

    let flags = ["--lz4", "--lzma", "--lzop", "--lrzip"];
    for flag in flags {
        let archive = format!(
            "compression/out_{}.pna",
            flag.trim_start_matches('-').replace('-', "_")
        );

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "c",
            archive.as_str(),
            "--overwrite",
            flag,
            "compression/input/raw/text.txt",
        ])
        .unwrap()
        .execute()
        .unwrap();

        assert!(
            fs::metadata(&archive).unwrap().is_file(),
            "archive not created: {archive}"
        );
    }
}
