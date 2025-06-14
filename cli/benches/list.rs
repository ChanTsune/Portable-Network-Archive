use clap::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use portable_network_archive::{cli, command::Command};

fn bench_list_normal(c: &mut Criterion) {
    c.bench_function("list_normal", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/zstd.pna"),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_list_solid(c: &mut Criterion) {
    c.bench_function("list_solid", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "--solid",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/solid_zstd.pna"
                ),
            ])
            .execute()
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_list_normal, bench_list_solid);
criterion_main!(benches);
