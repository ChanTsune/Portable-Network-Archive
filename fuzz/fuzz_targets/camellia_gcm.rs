#![no_main]

use libfuzzer_sys::fuzz_target;
use libpna::{
    CipherMode, Compression, Encryption, FileEntryBuilder, HashAlgorithm, ReadOptions, WriteOptions,
};
use std::io::prelude::*;

fuzz_target!(|data: &[u8]| {
    let write_option = WriteOptions::builder()
        .password(Some("password"))
        .encryption(Encryption::CAMELLIA)
        .cipher_mode(CipherMode::GCM)
        .compression(Compression::NO)
        .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
        .build();
    let mut builder = FileEntryBuilder::new_with_options("fuzz".into(), write_option).unwrap();
    builder.write_all(data).unwrap();
    let entry = builder.build().unwrap();
    let read_option = ReadOptions::with_password(Some("password"));
    let mut reader = entry.reader(read_option).unwrap();
    let mut buf = Vec::with_capacity(data.len());
    reader.read_to_end(&mut buf).unwrap();
    assert_eq!(data, buf);
});
