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
/// ```no_run
/// use libpna::{Chunk, ChunkType, RawChunk};
///
/// fn process_chunk<C: Chunk>(chunk: &C) {
///     println!("Chunk type: {:?}", chunk.ty());
///     println!("Data length: {}", chunk.length());
///     println!("CRC32: {:08x}", chunk.crc());
/// }
/// ```
pub trait Chunk {
    /// Returns the length of the chunk in bytes.
    ///
    /// # Returns
    ///
    /// The length of the chunk in bytes.
    #[inline]
    fn length(&self) -> u32 {
        self.data().len() as u32
    }

    /// Returns the type of the chunk.
    ///
    /// # Returns
    ///
    /// The type of the chunk.
    fn ty(&self) -> ChunkType;

    /// Returns the data of the chunk.
    ///
    /// # Returns
    ///
    /// A reference to the chunk data.
    fn data(&self) -> &[u8];

    /// Returns the CRC32 checksum of the chunk.
    ///
    /// # Returns
    ///
    /// The CRC32 checksum of the chunk.
    #[inline]
    fn crc(&self) -> u32 {
        let mut crc = Crc32::new();
        crc.update(&self.ty().0);
        crc.update(self.data());
        crc.finalize()
    }
}
