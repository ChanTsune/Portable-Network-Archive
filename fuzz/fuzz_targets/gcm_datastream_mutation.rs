#![no_main]

use libfuzzer_sys::fuzz_target;
use libpna::{
    Archive, CipherMode, Compression, Encryption, FileEntryBuilder, HashAlgorithm, ReadOptions,
    WriteOptions,
};
use std::io::prelude::*;

const PLAIN: &[u8] = b"aead datastream exercised through a mutated archive";
const PASSWORD: &str = "password";
const SIGNATURE_LEN: usize = 8;
const CHUNK_OVERHEAD: usize = 12;
/// Upper bound on the chunk walk. The archive is built here so its chunks are
/// well formed, but a fuzz target must not be able to spin.
const MAX_CHUNKS: usize = 4096;

/// Rewrites one byte of one of the entry's `FDAT` chunks and repairs the chunk
/// CRC.
///
/// Without the repair the chunk reader rejects the archive on its checksum and
/// the AEAD decoder is never reached, so every mutation would test CRC handling
/// instead of the datastream. `chunk_index` chooses which `FDAT` to hit: the
/// writer emits the stream header and the segments it frames as separate
/// chunks, so always taking the first would confine every mutation to the
/// header and never reach a segment's ciphertext or authentication tag.
fn mutate_fdat(archive: &mut [u8], chunk_index: usize, offset: usize, value: u8) -> bool {
    let mut fdat = Vec::new();
    let mut pos = SIGNATURE_LEN;
    for _ in 0..MAX_CHUNKS {
        if pos + CHUNK_OVERHEAD > archive.len() {
            break;
        }
        let length = u32::from_be_bytes(archive[pos..pos + 4].try_into().unwrap()) as usize;
        let ty = pos + 4;
        let body = ty + 4;
        let Some(crc_at) = body.checked_add(length) else {
            break;
        };
        if crc_at + 4 > archive.len() {
            break;
        }
        if &archive[ty..body] == b"FDAT" && length > 0 {
            fdat.push((ty, body, length, crc_at));
        }
        pos = crc_at + 4;
    }
    if fdat.is_empty() {
        return false;
    }
    let (ty, body, length, crc_at) = fdat[chunk_index % fdat.len()];
    archive[body + offset % length] ^= value;
    let crc = crc32fast::hash(&archive[ty..crc_at]);
    archive[crc_at..crc_at + 4].copy_from_slice(&crc.to_be_bytes());
    true
}

fuzz_target!(|data: (usize, usize, u8)| {
    let (chunk_index, offset, value) = data;
    if value == 0 {
        return;
    }
    let write_option = WriteOptions::builder()
        .password(Some(PASSWORD))
        .encryption(Encryption::AES)
        .cipher_mode(CipherMode::GCM)
        .compression(Compression::NO)
        .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
        .build();
    let mut builder = FileEntryBuilder::new_with_options("fuzz".into(), write_option).unwrap();
    builder.write_all(PLAIN).unwrap();
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    archive.add_entry(builder.build().unwrap()).unwrap();
    let mut bytes = archive.finalize().unwrap();

    if !mutate_fdat(&mut bytes, chunk_index, offset, value) {
        return;
    }

    let Ok(mut archive) = Archive::read_header_from_slice(&bytes) else {
        return;
    };
    for entry in archive
        .entries_slice()
        .extract_solid_entries(&ReadOptions::with_password(Some(PASSWORD)))
    {
        let Ok(entry) = entry else { return };
        let Ok(mut reader) = entry.reader(ReadOptions::with_password(Some(PASSWORD))) else {
            return;
        };
        let mut buf = Vec::new();
        match reader.read_to_end(&mut buf) {
            // Authentication either accepts the bytes that were written or
            // fails; it must never hand back plaintext that differs from them.
            Ok(_) => assert_eq!(buf, PLAIN),
            Err(_) => return,
        }
    }
});
