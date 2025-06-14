use clap::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use portable_network_archive::{cli, command::Command};

fn bench_create_with_split(c: &mut Criterion) {
    c.bench_function("create_with_split", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "c",
                concat!(
                    env!("CARGO_TARGET_TMPDIR"),
                    "/bench/create_with_split/store.pna"
                ),
                "--store",
                "--split",
                "3MB",
                "--overwrite",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw/"),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_split(c: &mut Criterion) {
    c.bench_function("split", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "split",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/store.pna"),
                "--overwrite",
                "--max-size",
                "3MB",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/split/"),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_extract_multipart(c: &mut Criterion) {
    c.bench_function("extract_multipart", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "x",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/multipart.part1.pna"
                ),
                "--overwrite",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/multipart/"),
            ])
            .execute()
            .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_create_with_split,
    bench_split,
    bench_extract_multipart
);
criterion_main!(benches);
