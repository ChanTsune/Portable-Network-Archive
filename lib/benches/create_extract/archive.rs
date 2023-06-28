extern crate test;

use crate::{bench_read_archive, bench_write_archive};
use libpna::{CipherMode, Compression, Encryption, WriteOption};
use test::Bencher;

#[bench]
fn write_store_archive(b: &mut Bencher) {
    bench_write_archive(b, WriteOption::builder());
}

#[bench]
fn read_store_archive(b: &mut Bencher) {
    bench_read_archive(b, WriteOption::builder());
}

#[bench]
fn write_zstd_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder.compression(Compression::ZStandard);
        builder
    });
}

#[bench]
fn read_zstd_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder.compression(Compression::ZStandard);
        builder
    });
}

#[bench]
fn write_deflate_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder.compression(Compression::Deflate);
        builder
    });
}

#[bench]
fn read_deflate_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder.compression(Compression::Deflate);
        builder
    });
}

#[bench]
fn write_lzma_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder.compression(Compression::XZ);
        builder
    });
}

#[bench]
fn read_lzma_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder.compression(Compression::XZ);
        builder
    });
}

#[bench]
fn write_aes_ctr_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn read_aes_ctr_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn write_aes_cbc_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn read_aes_cbc_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Aes)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn write_camellia_ctr_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn read_camellia_ctr_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CTR);
        builder
    });
}

#[bench]
fn write_camellia_cbc_archive(b: &mut Bencher) {
    bench_write_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}

#[bench]
fn read_camellia_cbc_archive(b: &mut Bencher) {
    bench_read_archive(b, {
        let mut builder = WriteOption::builder();
        builder
            .encryption(Encryption::Camellia)
            .cipher_mode(CipherMode::CBC);
        builder
    });
}
