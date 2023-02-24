extern crate test;

use crate::{bench_read_archive, bench_write_archive};
use libpna::{Compression, Decoder, Encoder, Encryption, Options};
use std::io::{self, Cursor};
use test::Bencher;

bench_write_archive!(write_store_archive, Options::default());

bench_read_archive!(read_store_archive, Options::default());

bench_write_archive!(
    write_zstd_archive,
    Options::default().compression(Compression::ZStandard)
);

bench_read_archive!(
    read_zstd_archive,
    Options::default().compression(Compression::ZStandard)
);

bench_write_archive!(
    write_deflate_archive,
    Options::default().compression(Compression::Deflate)
);

bench_read_archive!(
    read_deflate_archive,
    Options::default().compression(Compression::Deflate)
);

bench_write_archive!(
    write_lzma_archive,
    Options::default().compression(Compression::XZ)
);

bench_read_archive!(
    read_lzma_archive,
    Options::default().compression(Compression::XZ)
);

bench_write_archive!(
    write_aes_archive,
    Options::default().encryption(Encryption::Aes)
);

bench_read_archive!(
    read_aes_archive,
    Options::default().encryption(Encryption::Aes)
);

bench_write_archive!(
    write_camellia_archive,
    Options::default().encryption(Encryption::Camellia)
);

bench_read_archive!(
    read_camellia_archive,
    Options::default().encryption(Encryption::Camellia)
);
