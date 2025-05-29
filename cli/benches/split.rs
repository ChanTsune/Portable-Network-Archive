use clap::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use portable_network_archive::{cli, command};

fn bench_create_with_split(c: &mut Criterion) {
    c.bench_function("create_with_split", |b| {
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
                "../resources/test/raw/",
            ]))
            .unwrap()
        })
    });
}

fn bench_split(c: &mut Criterion) {
    c.bench_function("split", |b| {
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
    });
}

fn bench_extract_multipart(c: &mut Criterion) {
    c.bench_function("extract_multipart", |b| {
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
    });
}

criterion_group!(
    benches,
    bench_create_with_split,
    bench_split,
    bench_extract_multipart
);
criterion_main!(benches);
