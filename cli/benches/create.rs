use clap::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use portable_network_archive::{cli, command};

fn bench_store(c: &mut Criterion) {
    c.bench_function("store", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/store.pna"),
                "--store",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ]))
            .unwrap()
        })
    });
}

fn bench_zstd(c: &mut Criterion) {
    c.bench_function("zstd", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/zstd.pna"),
                "--zstd",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ]))
            .unwrap()
        })
    });
}

fn bench_deflate(c: &mut Criterion) {
    c.bench_function("deflate", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/deflate.pna"),
                "--deflate",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ]))
            .unwrap()
        })
    });
}

fn bench_xz(c: &mut Criterion) {
    c.bench_function("xz", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/xz.pna"),
                "--xz",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ]))
            .unwrap()
        })
    });
}

fn bench_zstd_keep_timestamp(c: &mut Criterion) {
    c.bench_function("zstd_keep_timestamp", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(
                    env!("CARGO_TARGET_TMPDIR"),
                    "/bench/zstd_keep_timestamp.pna"
                ),
                "--zstd",
                "--keep-timestamp",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ]))
            .unwrap()
        })
    });
}

fn bench_zstd_keep_permission(c: &mut Criterion) {
    c.bench_function("zstd_keep_permission", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(
                    env!("CARGO_TARGET_TMPDIR"),
                    "/bench/zstd_keep_permission.pna"
                ),
                "--zstd",
                "--keep-permission",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ]))
            .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_store,
    bench_zstd,
    bench_deflate,
    bench_xz,
    bench_zstd_keep_timestamp,
    bench_zstd_keep_permission
);
criterion_main!(benches);
