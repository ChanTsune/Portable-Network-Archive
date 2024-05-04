#![feature(test)]
extern crate test;

use clap::Parser;
use portable_network_archive::{cli, command};
use test::Bencher;

#[bench]
fn create_with_split(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "c",
            &format!(
                "{}/bench/create_with_split/store.pna",
                env!("CARGO_TARGET_TMPDIR")
            ),
            "--store",
            "--split",
            "3MB",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}

#[bench]
fn split(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "split",
            "../resources/test/store.pna",
            "--overwrite",
            "--max-size",
            "3MB",
            "--out-dir",
            &format!("{}/bench/split/", env!("CARGO_TARGET_TMPDIR")),
        ]))
        .unwrap()
    })
}

#[bench]
fn extract_multipart(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "x",
            "../resources/test/multipart.part1.pna",
            "--overwrite",
            "--out-dir",
            &format!("{}/bench/multipart/", env!("CARGO_TARGET_TMPDIR")),
        ]))
        .unwrap()
    })
}
