use super::{ChunkType, Crc32};

/// The smallest data unit in PNA.
pub trait Chunk {
    /// Length of data in bytes.
    #[inline]
    fn length(&self) -> u32 {
        self.data().len() as u32
    }
    /// Type of chunk.
    fn ty(&self) -> ChunkType;
    /// Data of chunk.
    fn data(&self) -> &[u8];
    /// CRC32 of chunk type and data.
    #[inline]
    fn crc(&self) -> u32 {
        let mut crc = Crc32::new();
        crc.update(&self.ty().0);
        crc.update(self.data());
        crc.finalize()
    }
}
