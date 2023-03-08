#![feature(test)]
extern crate test;

use clap::Parser;
use portable_network_archive::command;
use test::Bencher;

#[bench]
fn store(b: &mut Bencher) {
    b.iter(|| {
        command::entry(command::Args::parse_from([
            "pna",
            "-x",
            "../resources/test/empty.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
        ]))
        .unwrap()
    })
}

#[bench]
fn zstd(b: &mut Bencher) {
    b.iter(|| {
        command::entry(command::Args::parse_from([
            "pna",
            "-x",
            "../resources/test/zstd.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
        ]))
        .unwrap()
    })
}

#[bench]
fn deflate(b: &mut Bencher) {
    b.iter(|| {
        command::entry(command::Args::parse_from([
            "pna",
            "-x",
            "../resources/test/deflate.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
        ]))
        .unwrap()
    })
}

#[bench]
fn lzma(b: &mut Bencher) {
    b.iter(|| {
        command::entry(command::Args::parse_from([
            "pna",
            "-x",
            "../resources/test/lzma.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
        ]))
        .unwrap()
    })
}
