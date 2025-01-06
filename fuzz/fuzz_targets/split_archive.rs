#![no_main]

use libfuzzer_sys::fuzz_target;
use libpna::{Archive, Compression, EntryBuilder, EntryPart, ReadOptions, WriteOptions};
use std::io::prelude::*;

fuzz_target!(|data: (&[u8], usize)| {
    let (data, split_size) = data;
    let write_option = WriteOptions::builder().compression(Compression::No).build();
    let mut builder = EntryBuilder::new_file("fuzz".into(), write_option).unwrap();
    builder.write_all(data).unwrap();
    let entry = builder.build().unwrap();
    let entry_part = EntryPart::from(entry);
    let mut entry_part = entry_part.as_ref();
    let mut parts = Vec::new();
    loop {
        match entry_part.try_split(split_size) {
            Ok((write_part, Some(remaining_part))) => {
                parts.push(write_part);
                entry_part = remaining_part;
            }
            Ok((write_part, None)) => {
                parts.push(write_part);
                break;
            }
            Err(_) => return,
        }
    }

    let mut archive = Archive::write_header(Vec::new()).unwrap();
    for part in parts {
        archive.add_entry_part(part).unwrap();
    }
    let archive_bytes = archive.finalize().unwrap();
    let mut archive = Archive::read_header_from_slice(&archive_bytes).unwrap();

    for entry in archive.entries_slice().extract_solid_entries(None) {
        let entry = entry.unwrap();
        let read_option = ReadOptions::builder().build();
        let mut reader = entry.reader(read_option).unwrap();
        let mut buf = Vec::with_capacity(data.len());
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(data, buf);
    }
});
