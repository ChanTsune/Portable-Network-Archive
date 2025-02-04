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
            &format!("{}/bench/store.pna", env!("CARGO_TARGET_TMPDIR")),
            "--store",
            "--overwrite",
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
            &format!("{}/bench/zstd.pna", env!("CARGO_TARGET_TMPDIR")),
            "--zstd",
            "--overwrite",
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
            &format!("{}/bench/deflate.pna", env!("CARGO_TARGET_TMPDIR")),
            "--deflate",
            "--overwrite",
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
            &format!("{}/bench/xz.pna", env!("CARGO_TARGET_TMPDIR")),
            "--xz",
            "--overwrite",
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
            &format!(
                "{}/bench/zstd_keep_timestamp.pna",
                env!("CARGO_TARGET_TMPDIR")
            ),
            "--zstd",
            "--keep-timestamp",
            "--overwrite",
            "../resources/test/raw/",
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
            "c",
            &format!(
                "{}/bench/zstd_keep_permission.pna",
                env!("CARGO_TARGET_TMPDIR")
            ),
            "--zstd",
            "--keep-permission",
            "--overwrite",
            "../resources/test/raw/",
        ]))
        .unwrap()
    })
}
