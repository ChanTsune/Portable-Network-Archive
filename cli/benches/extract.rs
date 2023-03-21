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
            "x",
            "../resources/test/store.pna",
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
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "x",
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
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "x",
            "../resources/test/deflate.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
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
            "x",
            "../resources/test/lzma.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
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
            "x",
            "../resources/test/zstd_keep_timestamp.pna",
            "--overwrite",
            "--out-dir",
            "../out/",
        ]))
        .unwrap()
    })
}

#[bench]
fn zstd_keep_permission(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "x",
            "../resources/test/zstd_keep_permission.pna",
            "--overwrite",
            "--keep-permission",
            "--out-dir",
            "../out/",
        ]))
        .unwrap()
    })
}
