use clap::Parser;
use criterion::{Criterion, criterion_group, criterion_main};
use portable_network_archive::cli;

fn bench_list_normal(c: &mut Criterion) {
    c.bench_function("list_normal", |b| {
        b.iter(|| {
            cli::Cli::parse_from([
                "pna",
                "--quiet",
                "ls",
                "--file",
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
                "--file",
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
                "--file",
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
                "--file",
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
                "--file",
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
                "--file",
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
                "--file",
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
