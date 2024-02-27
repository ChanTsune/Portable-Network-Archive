#![feature(test)]
extern crate test;

use clap::Parser;
use portable_network_archive::{cli, command};
use test::Bencher;

#[bench]
fn regular(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "ls",
            "../resources/test/zstd.pna",
        ]))
        .unwrap()
    })
}

#[bench]
fn solid(b: &mut Bencher) {
    b.iter(|| {
        command::entry(cli::Cli::parse_from([
            "pna",
            "--quiet",
            "ls",
            "--solid",
            "../resources/test/solid.pna",
        ]))
        .unwrap()
    })
}
