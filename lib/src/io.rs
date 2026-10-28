//! I/O primitives for reading and writing PNA archives.

mod chunk;
mod chunks;
mod signature;

use crate::{Chunk, ChunkType};
use std::io;

pub use chunk::{read_chunk, skip_chunk, write_chunk};
pub use chunks::read_chunks;
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
