use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use libpna::{
    Archive, CipherMode, Compression, Encryption, EntryBuilder, ReadEntry, ReadOptions,
    WriteOptions, WriteOptionsBuilder,
};
use std::io::{self, prelude::*};

fn bench_write_archive(b: &mut Bencher, mut options: WriteOptionsBuilder) {
    let buf = [24; 1111];
    b.iter(|| {
        let mut vec = Vec::with_capacity(10000);
        let mut writer = Archive::write_header(&mut vec).unwrap();
        writer
            .add_entry({
                let mut builder = EntryBuilder::new_file(
                    "bench".into(),
                    options.password(Some("password")).build(),
                )
                .unwrap();
                builder.write_all(&buf).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        writer.finalize().unwrap();
    })
}

fn bench_read_archive(b: &mut Bencher, mut options: WriteOptionsBuilder) {
    let buf = [24; 1111];
    let mut writer = Archive::write_header(Vec::with_capacity(10000)).unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("bench".into(), options.password(Some("password")).build())
                    .unwrap();
            builder.write_all(&buf).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    let vec = writer.finalize().unwrap();

    b.iter(|| {
        let mut reader = Archive::read_header(vec.as_slice()).unwrap();
        for item in reader.entries_skip_solid() {
            let mut buf = Vec::with_capacity(1000);
            item.unwrap()
                .reader(ReadOptions::with_password(Some("password")))
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
        }
    })
}

fn bench_read_archive_from_slice(b: &mut Bencher, mut options: WriteOptionsBuilder) {
    let buf = [24; 1111];
    let mut writer = Archive::write_header(Vec::with_capacity(10000)).unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("bench".into(), options.password(Some("password")).build())
                    .unwrap();
            builder.write_all(&buf).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    let vec = writer.finalize().unwrap();

    b.iter(|| {
        let mut reader = Archive::read_header_from_slice(vec.as_slice()).unwrap();
        for item in reader.entries_slice() {
            let mut buf = Vec::with_capacity(1000);
            match item.unwrap() {
                ReadEntry::Solid(_) => (),
                ReadEntry::Normal(item) => {
                    item.reader(ReadOptions::with_password(Some("password")))
                        .unwrap()
                        .read_to_end(&mut buf)
                        .unwrap();
                }
            }
        }
    })
}

fn bench_write_store_archive(c: &mut Criterion) {
    c.bench_function("write_store_archive", |b| {
        bench_write_archive(b, WriteOptions::builder());
    });
}

fn bench_read_store_archive(c: &mut Criterion) {
    c.bench_function("read_store_archive", |b| {
        bench_read_archive(b, WriteOptions::builder());
    });
}

fn bench_read_store_archive_from_slice(c: &mut Criterion) {
    c.bench_function("read_store_archive_from_slice", |b| {
        bench_read_archive_from_slice(b, WriteOptions::builder());
    });
}

fn bench_write_zstd_archive(c: &mut Criterion) {
    c.bench_function("write_zstd_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::ZStandard);
            builder
        });
    });
}

fn bench_read_zstd_archive(c: &mut Criterion) {
    c.bench_function("read_zstd_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::ZStandard);
            builder
        });
    });
}

fn bench_read_zstd_archive_from_slice(c: &mut Criterion) {
    c.bench_function("read_zstd_archive_from_slice", |b| {
        bench_read_archive_from_slice(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::ZStandard);
            builder
        });
    });
}

fn bench_write_deflate_archive(c: &mut Criterion) {
    c.bench_function("write_deflate_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::Deflate);
            builder
        });
    });
}

fn bench_read_deflate_archive(c: &mut Criterion) {
    c.bench_function("read_deflate_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::Deflate);
            builder
        });
    });
}

fn bench_read_deflate_archive_from_slice(c: &mut Criterion) {
    c.bench_function("read_deflate_archive_from_slice", |b| {
        bench_read_archive_from_slice(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::Deflate);
            builder
        });
    });
}

fn bench_write_xz_archive(c: &mut Criterion) {
    c.bench_function("write_xz_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::XZ);
            builder
        });
    });
}

fn bench_read_xz_archive(c: &mut Criterion) {
    c.bench_function("read_xz_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::XZ);
            builder
        });
    });
}

fn bench_read_xz_archive_from_slice(c: &mut Criterion) {
    c.bench_function("read_xz_archive_from_slice", |b| {
        bench_read_archive_from_slice(b, {
            let mut builder = WriteOptions::builder();
            builder.compression(Compression::XZ);
            builder
        });
    });
}

fn bench_write_aes_ctr_archive(c: &mut Criterion) {
    c.bench_function("write_aes_ctr_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CTR);
            builder
        });
    });
}

fn bench_read_aes_ctr_archive(c: &mut Criterion) {
    c.bench_function("read_aes_ctr_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CTR);
            builder
        });
    });
}

fn bench_write_aes_cbc_archive(c: &mut Criterion) {
    c.bench_function("write_aes_cbc_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC);
            builder
        });
    });
}

fn bench_read_aes_cbc_archive(c: &mut Criterion) {
    c.bench_function("read_aes_cbc_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC);
            builder
        });
    });
}

fn bench_write_camellia_ctr_archive(c: &mut Criterion) {
    c.bench_function("write_camellia_ctr_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CTR);
            builder
        });
    });
}

fn bench_read_camellia_ctr_archive(c: &mut Criterion) {
    c.bench_function("read_camellia_ctr_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CTR);
            builder
        });
    });
}

fn bench_write_camellia_cbc_archive(c: &mut Criterion) {
    c.bench_function("write_camellia_cbc_archive", |b| {
        bench_write_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC);
            builder
        });
    });
}

fn bench_read_camellia_cbc_archive(c: &mut Criterion) {
    c.bench_function("read_camellia_cbc_archive", |b| {
        bench_read_archive(b, {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC);
            builder
        });
    });
}

fn bench_write_empty_archive(c: &mut Criterion) {
    c.bench_function("write_empty_archive", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(1000);
            let writer = Archive::write_header(&mut vec).expect("failed to write header");
            writer.finalize().expect("failed to finalize");
        })
    });
}

fn bench_read_empty_archive(c: &mut Criterion) {
    c.bench_function("read_empty_archive", |b| {
        let writer =
            Archive::write_header(Vec::with_capacity(1000)).expect("failed to write header");
        let vec = writer.finalize().expect("failed to finalize");

        b.iter(|| {
            let mut reader = Archive::read_header(vec.as_slice()).expect("failed to read header");
            for entry in reader.entries_skip_solid() {
                let item = entry.expect("failed to read entry");
                io::read_to_string(
                    item.reader(ReadOptions::builder().build())
                        .expect("failed to read entry"),
                )
                .expect("failed to make string");
            }
        })
    });
}

criterion_group!(
    benches,
    bench_write_store_archive,
    bench_read_store_archive,
    bench_read_store_archive_from_slice,
    bench_write_zstd_archive,
    bench_read_zstd_archive,
    bench_read_zstd_archive_from_slice,
    bench_write_deflate_archive,
    bench_read_deflate_archive,
    bench_read_deflate_archive_from_slice,
    bench_write_xz_archive,
    bench_read_xz_archive,
    bench_read_xz_archive_from_slice,
    bench_write_aes_ctr_archive,
    bench_read_aes_ctr_archive,
    bench_write_aes_cbc_archive,
    bench_read_aes_cbc_archive,
    bench_write_camellia_ctr_archive,
    bench_read_camellia_ctr_archive,
    bench_write_camellia_cbc_archive,
    bench_read_camellia_cbc_archive,
    bench_write_empty_archive,
    bench_read_empty_archive
);
criterion_main!(benches);
