//! AEAD chunked encryption writer.
//!
//! **Status: prototype skeleton (2026-04-24).**
//!
//! Per spec §7.5.3, encoders MUST chunk plaintext into uniform-size segments
//! (recommended 64 KiB) and emit each as one FDAT/SDAT chunk whose data is
//! `(ciphertext || 16-byte tag)`. The final chunk preceding FEND/SEND MUST
//! be marked with the AAD `is_final_chunk` flag = `0x01`.

#![allow(dead_code)] // skeleton

use std::io::{self, Write};

use super::AeadContext;
use super::read::AeadAlgorithm;

/// Default chunk plaintext size (64 KiB).
///
/// Matches age STREAM construction (C2SP/age.md §"Payload"). Sweet spot per
/// Tink documentation: small enough to bound streaming memory, large enough
/// to amortize 16-byte tag overhead (~0.024% per chunk).
pub(crate) const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// AEAD chunked encryption writer.
///
/// **Not yet implemented.**
pub(crate) struct AeadWriter<W: Write> {
    _inner: W,
    _ctx: AeadContext,
    _algorithm: AeadAlgorithm,
    _chunk_buffer: Vec<u8>,
    _chunk_size: usize,
    _chunk_index: u32,
}

impl<W: Write> AeadWriter<W> {
    /// Wrap an inner writer with AEAD chunked encryption.
    ///
    /// Buffers up to `chunk_size` bytes of plaintext, then encrypts and emits
    /// one chunk's `(ciphertext || tag)` to the inner writer.
    pub(crate) fn new(
        _inner: W,
        _ctx: AeadContext,
        _algorithm: AeadAlgorithm,
        _chunk_size: usize,
    ) -> Self {
        unimplemented!("AeadWriter not yet implemented; see README.md")
    }

    /// Finalize the writer, emitting the last chunk with `is_final_chunk = 0x01`.
    ///
    /// MUST be called before dropping. Failure to call `finish` is a bug:
    /// the last chunk will not have the final flag set, causing decoders
    /// to report a truncation error.
    pub(crate) fn finish(self) -> io::Result<W> {
        unimplemented!()
    }
}

impl<W: Write> Write for AeadWriter<W> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        // TODO: per spec §7.5.3:
        //   1. Append _buf to _chunk_buffer.
        //   2. While _chunk_buffer.len() >= _chunk_size:
        //      a. Take first _chunk_size bytes as plaintext.
        //      b. Compute AAD: _ctx.build_aad(_chunk_index, false).
        //      c. Compute nonce: _ctx.build_nonce(_chunk_index).
        //      d. Encrypt under chosen algorithm; get (ciphertext, tag).
        //      e. Emit one chunk to _inner: data = (ciphertext || tag).
        //      f. _chunk_index += 1.
        //   3. Return _buf.len() as bytes written.
        unimplemented!()
    }

    fn flush(&mut self) -> io::Result<()> {
        // Forward to inner; do NOT emit partial chunks here.
        // Partial-chunk emission only happens in finish() with is_final flag.
        unimplemented!()
    }
}
