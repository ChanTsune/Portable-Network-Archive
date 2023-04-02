mod crc;
mod read;
mod traits;
mod types;
mod write;

use self::crc::Crc32;
pub(crate) use self::{read::ChunkReader, write::ChunkWriter};
pub use self::{traits::*, types::*};
use std::{mem, ops::Deref};

pub(crate) const MIN_CHUNK_BYTES_SIZE: usize = 12;

pub(crate) trait ChunkExt: Chunk {
    /// byte size of chunk
    fn bytes_len(&self) -> usize {
        mem::align_of::<u32>() + self.ty().len() + self.data().len() + mem::align_of::<u32>()
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
    pub fn from_data(ty: ChunkType, data: Vec<u8>) -> Self {
        let chunk = (ty, data);
        Self {
            length: chunk.length(),
            crc: chunk.crc(),
            ty: chunk.0,
            data: chunk.1,
        }
    }
}

impl Chunk for RawChunk {
    fn length(&self) -> u32 {
        self.length
    }

    fn ty(&self) -> ChunkType {
        self.ty
    }

    fn data(&self) -> &[u8] {
        &self.data
    }

    fn crc(&self) -> u32 {
        self.crc
    }
}

impl<T: Deref<Target = [u8]>> Chunk for (ChunkType, T) {
    fn ty(&self) -> ChunkType {
        self.0
    }

    fn data(&self) -> &[u8] {
        self.1.deref()
    }
}

/// Convert the provided `Chunk` instance into a `Vec<u8>`.
///
/// # Arguments
///
/// * `chunk` - A `Chunk` instance to be converted into a byte vector.
///
/// # Returns
///
/// A `Vec<u8>` containing the converted `Chunk` data.
///
pub(crate) fn chunk_to_bytes(chunk: impl Chunk) -> Vec<u8> {
    let mut vec = Vec::with_capacity(chunk.bytes_len());
    vec.extend_from_slice(&chunk.length().to_be_bytes());
    vec.extend_from_slice(&chunk.ty().0);
    vec.extend_from_slice(chunk.data());
    vec.extend_from_slice(&chunk.crc().to_be_bytes());
    vec
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

        let bytes = chunk_to_bytes(chunk);

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
