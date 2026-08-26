//! Chunk trait defining the interface for PNA archive chunks.

use crate::chunk::ChunkType;

/// A trait representing a chunk in a PNA archive.
///
/// A chunk is the basic unit of data storage in a PNA archive. Each chunk consists of:
/// - A length field (4 bytes)
/// - A chunk type (4 bytes)
/// - The chunk data (variable length)
/// - A CRC32 checksum (4 bytes)
///
/// This trait provides the basic interface for working with chunks in a PNA archive.
///
/// # Examples
///
/// ```
/// use libpna::{Chunk, ChunkType, RawChunk};
///
/// let chunk = RawChunk::from((ChunkType::FDAT, vec![1, 2, 3]));
/// assert_eq!(chunk.ty(), ChunkType::FDAT);
/// assert_eq!(chunk.length(), 3);
/// assert_eq!(chunk.data(), &[1, 2, 3]);
/// assert_eq!(chunk.crc(), 2776590148);
/// ```
pub trait Chunk {
    /// Returns the chunk's represented data length.
    ///
    /// The default implementation derives the length from [`Chunk::data`].
    /// Implementations that preserve an encoded chunk may override this method to
    /// return its stored `length` field.
    #[inline]
    fn length(&self) -> u32 {
        self.data().len() as u32
    }

    /// Returns the type of the chunk.
    fn ty(&self) -> ChunkType;

    /// Returns the data of the chunk.
    fn data(&self) -> &[u8];

    /// Returns the chunk's represented CRC32 checksum.
    ///
    /// The default implementation calculates the checksum over [`Chunk::ty`] and
    /// [`Chunk::data`]. Implementations that preserve an encoded chunk may override
    /// this method to return its stored `crc` field.
    #[inline]
    fn crc(&self) -> u32 {
        crate::format::chunk_crc(self.ty().as_bytes(), self.data())
    }
}
