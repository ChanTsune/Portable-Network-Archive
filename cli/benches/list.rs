use clap::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use portable_network_archive::{cli, command};

fn bench_normal(c: &mut Criterion) {
    c.bench_function("normal", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "../resources/test/zstd.pna",
            ]))
            .unwrap()
        })
    });
}

fn bench_solid(c: &mut Criterion) {
    c.bench_function("solid", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "--solid",
                "../resources/test/solid_zstd.pna",
            ]))
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_normal, bench_solid);
criterion_main!(benches);
