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
                &format!("{}/bench/store.pna", env!("CARGO_TARGET_TMPDIR")),
                "--store",
                "--overwrite",
                "../resources/test/raw/",
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
                &format!("{}/bench/zstd.pna", env!("CARGO_TARGET_TMPDIR")),
                "--zstd",
                "--overwrite",
                "../resources/test/raw/",
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
                &format!("{}/bench/deflate.pna", env!("CARGO_TARGET_TMPDIR")),
                "--deflate",
                "--overwrite",
                "../resources/test/raw/",
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
                &format!("{}/bench/xz.pna", env!("CARGO_TARGET_TMPDIR")),
                "--xz",
                "--overwrite",
                "../resources/test/raw/",
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
    });
}

fn bench_zstd_keep_permission(c: &mut Criterion) {
    c.bench_function("zstd_keep_permission", |b| {
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
