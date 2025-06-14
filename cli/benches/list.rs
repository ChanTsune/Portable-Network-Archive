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
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_dir.pna"
                ),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_list_normal_classify(c: &mut Criterion) {
    c.bench_function("list_normal_classify", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "--classify",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_dir.pna"
                ),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_list_normal_hide_control_chars(c: &mut Criterion) {
    c.bench_function("list_normal_hide_control_chars", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "-q",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_dir.pna"
                ),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_list_normal_table(c: &mut Criterion) {
    c.bench_function("list_normal_table", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "-l",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_dir.pna"
                ),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_list_normal_jsonl(c: &mut Criterion) {
    c.bench_function("list_normal_jsonl", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "--format",
                "jsonl",
                "--unstable",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_dir.pna"
                ),
            ])
            .execute()
            .unwrap()
        })
    });
}

fn bench_list_normal_tree(c: &mut Criterion) {
    c.bench_function("list_normal_tree", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "--format",
                "tree",
                "--unstable",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_dir.pna"
                ),
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

criterion_group!(
    benches,
    bench_list_normal,
    bench_list_normal_classify,
    bench_list_normal_hide_control_chars,
    bench_list_normal_table,
    bench_list_normal_jsonl,
    bench_list_normal_tree,
    bench_list_solid,
);
criterion_main!(benches);
