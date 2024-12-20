#![no_main]

use libfuzzer_sys::fuzz_target;
use libpna::{CipherMode, Compression, Encryption, EntryBuilder, ReadOptions, WriteOptions};
use std::io::prelude::*;

fuzz_target!(|data: &[u8]| {
    let write_option = WriteOptions::builder()
        .password(Some("password"))
        .encryption(Encryption::Aes)
        .cipher_mode(CipherMode::CBC)
        .compression(Compression::No)
        .build();
    let mut builder = EntryBuilder::new_file("fuzz".into(), write_option).unwrap();
    builder.write_all(data).unwrap();
    let entry = builder.build().unwrap();
    let read_option = ReadOptions::with_password(Some("password"));
    let mut reader = entry.reader(read_option).unwrap();
    let mut buf = Vec::with_capacity(data.len());
    reader.read_to_end(&mut buf).unwrap();
    assert_eq!(data, buf);
});
