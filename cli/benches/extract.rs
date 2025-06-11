use clap::Parser;
use criterion::{criterion_group, criterion_main, Criterion};
use portable_network_archive::{cli, command};

fn bench_store(c: &mut Criterion) {
    c.bench_function("store", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "x",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/store.pna"),
                "--overwrite",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/store/"),
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
                "x",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/zstd.pna"),
                "--overwrite",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/zstd/"),
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
                "x",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/deflate.pna"),
                "--overwrite",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/deflate/"),
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
                "x",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/xz.pna"),
                "--overwrite",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/xz/"),
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
                "x",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_timestamp.pna"
                ),
                "--overwrite",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/zstd_keep_timestamp/"),
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
                "x",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_permission.pna"
                ),
                "--overwrite",
                "--keep-permission",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/zstd_keep_permission/"),
            ]))
            .unwrap()
        })
    });
}

fn bench_zstd_keep_xattr(c: &mut Criterion) {
    c.bench_function("zstd_keep_xattr", |b| {
        b.iter(|| {
            command::entry(cli::Cli::parse_from([
                "pna",
                "--quiet",
                "x",
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/zstd_keep_xattr.pna"
                ),
                "--overwrite",
                #[cfg(not(target_os = "netbsd"))]
                "--keep-xattr",
                "--out-dir",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/bench/zstd_keep_xattr/"),
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
    bench_zstd_keep_permission,
    bench_zstd_keep_xattr
);
criterion_main!(benches);
