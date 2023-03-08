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
            "-c",
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
        command::entry(command::Args::parse_from([
            "pna",
            "-c",
            "../out/zstd.pna",
            "--store",
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
        command::entry(command::Args::parse_from([
            "pna",
            "-c",
            "../out/deflate.pna",
            "--store",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}

#[bench]
fn lzma(b: &mut Bencher) {
    b.iter(|| {
        command::entry(command::Args::parse_from([
            "pna",
            "-c",
            "../out/lzma.pna",
            "--store",
            "--overwrite",
            "-r",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}
