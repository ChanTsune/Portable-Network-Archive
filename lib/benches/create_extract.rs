#![feature(test)]
extern crate test;

use libpna::{
    Archive, CipherMode, Compression, Encryption, EntryBuilder, ReadEntry, ReadOptions,
    WriteOptions, WriteOptionsBuilder,
};
use std::io::{self, prelude::*};
use test::Bencher;

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

#[bench]
fn write_store_archive(b: &mut Bencher) {
    bench_write_archive(b, WriteOptions::builder());
}

#[bench]
fn read_store_archive(b: &mut Bencher) {
    bench_read_archive(b, WriteOptions::builder());
}

#[bench]
fn read_store_archive_from_slice(b: &mut Bencher) {
    bench_read_archive_from_slice(b, WriteOptions::builder());
}

#[bench]
fn write_zstd_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::ZStandard);
        builder
    });
}

#[bench]
fn read_zstd_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::ZStandard);
        builder
    });
}

#[bench]
fn read_zstd_archive_from_slice(b: &mut Bencher) {
    bench_read_archive_from_slice(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::ZStandard);
        builder
    });
}

#[bench]
fn write_deflate_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::Deflate);
        builder
    });
}

#[bench]
fn read_deflate_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::Deflate);
        builder
    });
}

#[bench]
fn read_deflate_archive_from_slice(b: &mut Bencher) {
    bench_read_archive_from_slice(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::Deflate);
        builder
    });
}

#[bench]
fn write_lzma_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::XZ);
        builder
    });
}

#[bench]
fn read_lzma_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::XZ);
        builder
    });
}

#[bench]
fn read_lzma_archive_from_slice(b: &mut Bencher) {
    bench_read_archive_from_slice(b, {
        let mut builder = WriteOptions::builder();
        builder.compression(Compression::XZ);
        builder
    });
}

#[bench]
fn write_aes_ctr_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn read_aes_ctr_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn write_aes_cbc_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn read_aes_cbc_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn write_camellia_ctr_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn read_camellia_ctr_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn write_camellia_cbc_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn read_camellia_cbc_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn write_empty_archive(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::with_capacity(1000);
        let writer = Archive::write_header(&mut vec).expect("failed to write header");
        writer.finalize().expect("failed to finalize");
    })
}

#[bench]
fn read_empty_archive(b: &mut Bencher) {
    let writer = Archive::write_header(Vec::with_capacity(1000)).expect("failed to write header");
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
}
