//! AEAD chunked decryption reader.
//!
//! **Status: prototype skeleton (2026-04-24).**
//!
//! Per spec §7.5.2, decoders MUST verify the tag of each FDAT/SDAT chunk
//! BEFORE releasing any plaintext to upper layers (Releasing Unverified
//! Plaintext defense, Bellare-Namprempre 2000).
//!
//! This reader operates chunk-by-chunk: it reads one full FDAT/SDAT chunk's
//! `(ciphertext || 16-byte tag)`, verifies the tag, decrypts, then exposes
//! the plaintext via the `Read` impl. If tag verification fails for any chunk,
//! all subsequent reads return [`std::io::ErrorKind::InvalidData`] and any
//! buffered plaintext from the failed entry is discarded.

#![allow(dead_code)] // skeleton

use std::io::{self, Read};

use super::AeadContext;

/// AEAD chunked decryption reader.
///
/// **Not yet implemented.**
pub(crate) struct AeadReader<R: Read> {
    _inner: R,
    _ctx: AeadContext,
    _algorithm: AeadAlgorithm,
    _chunk_index: u32,
    _pending_plaintext: Vec<u8>,
    _finished: bool,
    _last_chunk_seen: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AeadAlgorithm {
    Aes256Gcm,
    Camellia256Gcm,
}

impl<R: Read> AeadReader<R> {
    /// Wrap an inner reader, providing AEAD-decrypted plaintext via `Read`.
    pub(crate) fn new(_inner: R, _ctx: AeadContext, _algorithm: AeadAlgorithm) -> Self {
        unimplemented!("AeadReader not yet implemented; see README.md")
    }
}

impl<R: Read> Read for AeadReader<R> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // TODO: per spec §12.3.2/12.3.3:
        //   1. Read next chunk header (length, type, ...) from inner.
        //   2. Read full chunk data (12-byte nonce || ciphertext || 16-byte tag).
        //      Reject any chunk whose data length is < 28 bytes (12 + 0 + 16).
        //   3. Extract first 12 bytes as nonce; last 16 bytes as tag; middle as ciphertext.
        //   4. Compute AAD: ctx.build_aad(chunk_index, is_final_assumption).
        //   5. Verify tag using `aes_gcm::Aes256Gcm` or `camellia_gcm::CamelliaGcm256`.
        //   6. If verification fails, return InvalidData; do NOT release plaintext (RUP).
        //   7. If success, append decrypted plaintext to pending buffer; serve from buffer.
        //   8. On reaching FEND/SEND without seeing Final-chunk flag = 0x01 authenticated,
        //      return InvalidData (truncation).
        unimplemented!()
    }
}
