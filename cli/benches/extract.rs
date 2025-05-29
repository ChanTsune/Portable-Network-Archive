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
                "../resources/test/store.pna",
                "--overwrite",
                "--out-dir",
                &format!("{}/bench/store/", env!("CARGO_TARGET_TMPDIR")),
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
                "../resources/test/zstd.pna",
                "--overwrite",
                "--out-dir",
                &format!("{}/bench/zstd/", env!("CARGO_TARGET_TMPDIR")),
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
                "../resources/test/deflate.pna",
                "--overwrite",
                "--out-dir",
                &format!("{}/bench/deflate/", env!("CARGO_TARGET_TMPDIR")),
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
                "../resources/test/xz.pna",
                "--overwrite",
                "--out-dir",
                &format!("{}/bench/xz/", env!("CARGO_TARGET_TMPDIR")),
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
                "../resources/test/zstd_keep_timestamp.pna",
                "--overwrite",
                "--out-dir",
                &format!("{}/bench/zstd_keep_timestamp/", env!("CARGO_TARGET_TMPDIR")),
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
                "../resources/test/zstd_keep_permission.pna",
                "--overwrite",
                "--keep-permission",
                "--out-dir",
                &format!(
                    "{}/bench/zstd_keep_permission/",
                    env!("CARGO_TARGET_TMPDIR")
                ),
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
                "../resources/test/zstd_keep_xattr.pna",
                "--overwrite",
                #[cfg(not(target_os = "netbsd"))]
                "--keep-xattr",
                "--out-dir",
                &format!("{}/bench/zstd_keep_xattr/", env!("CARGO_TARGET_TMPDIR")),
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
