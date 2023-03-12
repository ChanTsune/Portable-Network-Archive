extern crate test;

use crate::{bench_read_archive, bench_write_archive};
use libpna::{
    CipherMode, Compression, Decoder, Encoder, Encryption, ReadEntry, ReadOptionBuilder,
    WriteOptionBuilder,
};
use std::io::{Cursor, Read};
use test::Bencher;

bench_write_archive!(write_store_archive, WriteOptionBuilder::default());

bench_read_archive!(read_store_archive, WriteOptionBuilder::default());

bench_write_archive!(
    write_zstd_archive,
    WriteOptionBuilder::default().compression(Compression::ZStandard)
);

bench_read_archive!(
    read_zstd_archive,
    WriteOptionBuilder::default().compression(Compression::ZStandard)
);

bench_write_archive!(
    write_deflate_archive,
    WriteOptionBuilder::default().compression(Compression::Deflate)
);

bench_read_archive!(
    read_deflate_archive,
    WriteOptionBuilder::default().compression(Compression::Deflate)
);

bench_write_archive!(
    write_lzma_archive,
    WriteOptionBuilder::default().compression(Compression::XZ)
);

bench_read_archive!(
    read_lzma_archive,
    WriteOptionBuilder::default().compression(Compression::XZ)
);

bench_write_archive!(
    write_aes_ctr_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Aes)
        .cipher_mode(CipherMode::CTR)
);

bench_read_archive!(
    read_aes_ctr_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Aes)
        .cipher_mode(CipherMode::CTR)
);

bench_write_archive!(
    write_aes_cbc_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Aes)
        .cipher_mode(CipherMode::CBC)
);

bench_read_archive!(
    read_aes_cbc_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Aes)
        .cipher_mode(CipherMode::CBC)
);

bench_write_archive!(
    write_camellia_ctr_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Camellia)
        .cipher_mode(CipherMode::CTR)
);

bench_read_archive!(
    read_camellia_ctr_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Camellia)
        .cipher_mode(CipherMode::CTR)
);

bench_write_archive!(
    write_camellia_cbc_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Camellia)
        .cipher_mode(CipherMode::CBC)
);

bench_read_archive!(
    read_camellia_cbc_archive,
    WriteOptionBuilder::default()
        .encryption(Encryption::Camellia)
        .cipher_mode(CipherMode::CBC)
);
