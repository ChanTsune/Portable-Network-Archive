mod crc;
mod read;
mod types;
mod write;

use crc::Crc32;
pub(crate) use read::ChunkReader;
use std::ops::Deref;
pub use types::*;
pub(crate) use write::ChunkWriter;

pub(crate) trait Chunk {
    fn length(&self) -> u32 {
        self.data().len() as u32
    }
    fn ty(&self) -> &ChunkType;
    fn data(&self) -> &[u8];
    fn crc(&self) -> u32 {
        let mut crc = Crc32::new();
        crc.update(&self.ty().0);
        crc.update(self.data());
        crc.finalize()
    }
}

pub(crate) type ChunkImpl = (ChunkType, Vec<u8>);
pub(crate) type Chunks = Vec<ChunkImpl>;

impl<T: Deref<Target = [u8]>> Chunk for (ChunkType, T) {
    fn ty(&self) -> &ChunkType {
        &self.0
    }

    fn data(&self) -> &[u8] {
        self.1.deref()
    }
}

pub(crate) fn create_chunk_data_ahed(major: u8, minor: u8, archive_number: u32) -> [u8; 8] {
    let mut data = [0; 8];
    data[0] = major;
    data[1] = minor;
    data[2..4].copy_from_slice(&[0, 0]);
    data[4..8].copy_from_slice(&archive_number.to_be_bytes());
    data
}

pub(crate) fn chunk_to_bytes(chunk: impl Chunk) -> Vec<u8> {
    let mut vec = Vec::with_capacity(12usize + chunk.length() as usize);
    vec.extend_from_slice(&chunk.length().to_be_bytes());
    vec.extend_from_slice(&chunk.ty().0);
    vec.extend_from_slice(chunk.data());
    vec.extend_from_slice(&chunk.crc().to_be_bytes());
    vec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create_chunk_data_ahed;

    #[test]
    fn ahed() {
        assert_eq!([0u8, 0, 0, 0, 0, 0, 0, 0], create_chunk_data_ahed(0, 0, 0));
        assert_eq!([1u8, 2, 0, 0, 0, 0, 0, 3], create_chunk_data_ahed(1, 2, 3));
    }

    #[test]
    fn to_bytes() {
        assert_eq!(
            chunk_to_bytes((ChunkType::FDAT, "text data".as_bytes())),
            [
                0, 0, 0, 9, 70, 68, 65, 84, 116, 101, 120, 116, 32, 100, 97, 116, 97, 177, 70, 138,
                128
            ]
        )
    }
}
