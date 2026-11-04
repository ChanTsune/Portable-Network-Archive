//! I/O primitives for reading and writing PNA archives.

mod chunk;
mod signature;

use crate::{Chunk, ChunkType, MIN_CHUNK_BYTES_SIZE};
use std::io;

pub use chunk::{read_chunk, skip_chunk};
pub use signature::{is_pna, read_signature};

pub(crate) mod sealed {
    use std::io::Write;

    pub trait Sealed {}

    impl<W: Write> Sealed for W {}
}

// Kept out of the crate root: the blanket implementation puts its methods on
// every io::Write type, so a root re-export would inject them into method
// resolution for crates doing `use libpna::*`.
/// A sink for writing PNA chunks.
///
/// `io::Write` implementors serialize each chunk immediately as bytes; other
/// sinks may route or buffer chunk values without going through an
/// intermediate byte encoding. This trait is sealed and cannot be
/// implemented outside this crate.
pub trait WriteChunk: sealed::Sealed {
    /// Emits `chunk` to this sink and returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink fails to accept the chunk.
    fn write_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize>;

    /// Consumes this sink, emits the archive-end marker, and returns the
    /// finalized sink.
    ///
    /// Prefer [`Archive::finalize`](crate::Archive::finalize) when constructing
    /// an archive so the marker is emitted at the correct point.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink fails to write or finalize its output.
    #[inline]
    fn finalize_archive(mut self) -> io::Result<Self>
    where
        Self: Sized,
    {
        self.write_chunk((ChunkType::AEND, []))?;
        Ok(self)
    }

    // Named flush_chunks to avoid ambiguity with io::Write::flush when both
    // traits are in scope.
    /// Flushes any output buffered by this sink.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink fails to flush its buffered output.
    fn flush_chunks(&mut self) -> io::Result<()>;
}

impl<W: io::Write> WriteChunk for W {
    #[inline]
    fn write_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize> {
        crate::io::write_chunk(self, chunk)
    }

    #[inline]
    fn flush_chunks(&mut self) -> io::Result<()> {
        self.flush()
    }
}

/// Writes one PNA chunk to `writer`.
///
/// This function writes the values reported by [`Chunk::length`],
/// [`Chunk::ty`], [`Chunk::data`], and [`Chunk::crc`] without validating or
/// recalculating them. To derive the default length and CRC from a type and
/// data, pass a `(ChunkType, data)` tuple.
///
/// The archive signature and archive-level chunk ordering are not written or
/// interpreted. This function does not flush `writer`.
///
/// On success, the returned value is the number of bytes actually written,
/// based on the size of [`Chunk::data`] rather than [`Chunk::length`]. On
/// failure, a prefix of the chunk may already have been written.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] if the actual output length cannot
/// be represented by [`usize`], and any error produced by `writer`.
///
/// # Examples
///
/// ```
/// use libpna::{ChunkType, io};
///
/// let mut output = Vec::new();
/// let written = io::write_chunk(&mut output, (ChunkType::AEND, []))?;
///
/// assert_eq!(written, 12);
/// assert_eq!(output, [0, 0, 0, 0, b'A', b'E', b'N', b'D', 107, 246, 72, 109]);
/// # Ok::<(), std::io::Error>(())
/// ```
#[inline]
pub fn write_chunk<W: io::Write + ?Sized>(writer: &mut W, chunk: impl Chunk) -> io::Result<usize> {
    let length = chunk.length().to_be_bytes();
    let ty = chunk.ty();
    let data = chunk.data();
    let crc = chunk.crc().to_be_bytes();
    let bytes_len = MIN_CHUNK_BYTES_SIZE
        .checked_add(data.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chunk byte length overflow"))?;

    writer.write_all(&length)?;
    writer.write_all(ty.as_bytes())?;
    writer.write_all(data)?;
    writer.write_all(&crc)?;
    Ok(bytes_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn write_chunk_preserves_reported_fields() {
        struct StoredChunk;

        impl Chunk for StoredChunk {
            fn length(&self) -> u32 {
                1
            }

            fn ty(&self) -> ChunkType {
                ChunkType::FDAT
            }

            fn data(&self) -> &[u8] {
                b"abc"
            }

            fn crc(&self) -> u32 {
                0x0102_0304
            }
        }

        let mut output = Vec::new();
        let written = write_chunk(&mut output, StoredChunk).unwrap();

        assert_eq!(written, 15);
        assert_eq!(
            output,
            [
                0, 0, 0, 1, b'F', b'D', b'A', b'T', b'a', b'b', b'c', 1, 2, 3, 4
            ]
        );
    }
}
