#![no_main]

use libfuzzer_sys::fuzz_target;
use libpna::{Archive, ReadOptions};
use std::io::Read;

/// Bounds decompressed output per entry so a decompression bomb produces a
/// benign truncation instead of an OOM that would mask real crashes.
const MAX_ENTRY_BYTES: usize = 1 << 20;

fuzz_target!(|data: &[u8]| {
    let Ok(mut archive) = Archive::read_header_from_slice(data) else {
        return;
    };
    let options = ReadOptions::with_password(Some("password"));
    for entry in archive.entries_slice().extract_solid_entries(&options) {
        let Ok(entry) = entry else { continue };
        let Ok(mut reader) = entry.reader(options.clone()) else {
            continue;
        };
        let mut buf = [0u8; 4096];
        let mut total = 0usize;
        while total < MAX_ENTRY_BYTES {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => total += n,
            }
        }
    }
});
