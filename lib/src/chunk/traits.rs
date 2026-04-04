//! Chunk trait defining the interface for PNA archive chunks.

use super::{ChunkType, Crc32};

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
    /// Returns the length of the chunk's data payload in bytes.
    ///
    /// This value corresponds to the `length` field stored in the chunk structure
    /// and indicates the size of the data returned by the `data()` method.
    #[inline]
    fn length(&self) -> u32 {
        self.data().len() as u32
    }

    /// Returns the type of the chunk.
    fn ty(&self) -> ChunkType;

    /// Returns the data of the chunk.
    fn data(&self) -> &[u8];

    /// Returns the CRC32 checksum calculated over the chunk's type and data fields.
    #[inline]
    fn crc(&self) -> u32 {
        let mut crc = Crc32::new();
        crc.update(self.ty().as_bytes());
        crc.update(self.data());
        crc.finalize()
    }
}
