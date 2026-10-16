//! Chunk trait defining the interface for PNA archive chunks.

use crate::chunk::ChunkType;

/// A chunk in a PNA archive.
///
/// An encoded chunk consists of a 4-byte data length, a 4-byte type code,
/// the data, and a 4-byte CRC32 checksum of the type code and data.
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
