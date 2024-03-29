mod crc;
mod read;
mod traits;
mod types;
mod write;

use self::crc::Crc32;
pub(crate) use self::{read::ChunkReader, write::ChunkWriter};
pub use self::{traits::*, types::*};
use std::{
    io::{self, Write},
    mem,
    ops::Deref,
};

/// Minimum required size of bytes to represent [`Chunk`].
/// length:4 + chunk type:4 + data:0 + crc:4
pub const MIN_CHUNK_BYTES_SIZE: usize =
    mem::size_of::<u32>() + mem::size_of::<ChunkType>() + mem::size_of::<u32>();

pub(crate) trait ChunkExt: Chunk {
    /// byte size of chunk
    fn bytes_len(&self) -> usize {
        MIN_CHUNK_BYTES_SIZE + self.data().len()
    }

    /// check the chunk type is stream chunk
    fn is_stream_chunk(&self) -> bool {
        self.ty() == ChunkType::FDAT || self.ty() == ChunkType::SDAT
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        writer.write_all(&self.length().to_be_bytes())?;
        writer.write_all(&self.ty().0)?;
        writer.write_all(self.data())?;
        writer.write_all(&self.crc().to_be_bytes())?;
        Ok(self.bytes_len())
    }

    /// Convert the provided `Chunk` instance into a `Vec<u8>`.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the converted `Chunk` data.
    fn to_bytes(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(self.bytes_len());
        vec.extend_from_slice(&self.length().to_be_bytes());
        vec.extend_from_slice(&self.ty().0);
        vec.extend_from_slice(self.data());
        vec.extend_from_slice(&self.crc().to_be_bytes());
        vec
    }
}

impl<T> ChunkExt for T where T: Chunk {}

/// Represents a raw chunk
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RawChunk {
    pub(crate) length: u32,
    pub(crate) ty: ChunkType,
    pub(crate) data: Vec<u8>,
    pub(crate) crc: u32,
}

impl RawChunk {
    /// Create a new [`RawChunk`] from given [`ChunkType`] and [`Vec<u8>`].
    ///
    /// # Examples
    /// ```
    /// use libpna::{Chunk, ChunkType, RawChunk};
    ///
    /// let data = [0xAA, 0xBB, 0xCC, 0xDD];
    /// let chunk = RawChunk::from_data(ChunkType::FDAT, data);
    ///
    /// assert_eq!(chunk.length(), 4);
    /// assert_eq!(chunk.ty(), ChunkType::FDAT);
    /// assert_eq!(chunk.data(), &[0xAA, 0xBB, 0xCC, 0xDD]);
    /// assert_eq!(chunk.crc(), 1207118608);
    /// ```
    #[inline]
    pub fn from_data<T: Into<Vec<u8>>>(ty: ChunkType, data: T) -> Self {
        let data = data.into();
        let chunk = (ty, &data[..]);
        Self {
            length: chunk.length(),
            crc: chunk.crc(),
            ty,
            data,
        }
    }
}

impl Chunk for RawChunk {
    #[inline]
    fn length(&self) -> u32 {
        self.length
    }

    #[inline]
    fn ty(&self) -> ChunkType {
        self.ty
    }

    #[inline]
    fn data(&self) -> &[u8] {
        &self.data
    }

    #[inline]
    fn crc(&self) -> u32 {
        self.crc
    }
}

impl<T: Deref<Target = [u8]>> Chunk for (ChunkType, T) {
    #[inline]
    fn ty(&self) -> ChunkType {
        self.0
    }

    #[inline]
    fn data(&self) -> &[u8] {
        &self.1
    }
}

pub(crate) fn chunk_data_split(chunk: impl Chunk, mid: usize) -> (RawChunk, RawChunk) {
    let (first, last) = chunk.data().split_at(mid);
    (
        RawChunk::from_data(chunk.ty(), first.to_vec()),
        RawChunk::from_data(chunk.ty(), last.to_vec()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_bytes() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);

        let bytes = chunk.to_bytes();

        assert_eq!(
            bytes,
            vec![
                0x00, 0x00, 0x00, 0x04, // chunk length (4)
                0x46, 0x44, 0x41, 0x54, // chunk type ("FDAT")
                0xAA, 0xBB, 0xCC, 0xDD, // data bytes
                0x47, 0xf3, 0x2b, 0x10, // CRC32 (calculated from chunk type and data)
            ]
        );
    }

    #[test]
    fn data_split_at_zero() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);
        assert_eq!(
            chunk_data_split(chunk, 0),
            (
                RawChunk::from_data(ChunkType::FDAT, vec![]),
                RawChunk::from_data(ChunkType::FDAT, vec![0xAA, 0xBB, 0xCC, 0xDD]),
            )
        )
    }

    #[test]
    fn data_split_at_middle() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let chunk = RawChunk::from_data(ChunkType::FDAT, data);
        assert_eq!(
            chunk_data_split(chunk, 2),
            (
                RawChunk::from_data(ChunkType::FDAT, vec![0xAA, 0xBB]),
                RawChunk::from_data(ChunkType::FDAT, vec![0xCC, 0xDD]),
            )
        )
    }
}
