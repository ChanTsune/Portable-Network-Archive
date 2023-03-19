#![feature(test)]
extern crate test;

use clap::Parser;
use portable_network_archive::{cli, command};
use test::Bencher;

#[bench]
fn store(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "c",
            "../out/store.pna",
            "--store",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}

#[bench]
fn zstd(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "c",
            "../out/zstd.pna",
            "--zstd",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}

#[bench]
fn deflate(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "c",
            "../out/deflate.pna",
            "--deflate",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}

#[bench]
fn xz(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "c",
            "../out/lzma.pna",
            "--xz",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}

#[bench]
fn zstd_keep_timestamp(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "c",
            "../out/zstd_keep_timestamp.pna",
            "--zstd",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}
