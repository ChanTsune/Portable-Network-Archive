//! Chunk reading and deserialization from byte streams and slices.

use crate::chunk::{ChunkType, MIN_CHUNK_BYTES_SIZE};
use std::{
    io::{self, Read, Seek, SeekFrom},
    mem,
};

pub(crate) struct ChunkReader<R> {
    pub(crate) r: R,
}

impl<R> ChunkReader<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self { r: reader }
    }
}

impl<R: Read + Seek> ChunkReader<R> {
    pub(crate) fn skip_chunk(&mut self) -> io::Result<(ChunkType, usize)> {
        // read chunk length
        let mut length = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        self.r.read_exact(&mut ty)?;

        // skip chunk data
        self.r.seek(SeekFrom::Current(length.into()))?;

        // skip crc sum
        self.r
            .seek(SeekFrom::Current(mem::size_of::<u32>() as i64))?;

        Ok((
            ChunkType::new(ty)?,
            MIN_CHUNK_BYTES_SIZE
                .checked_add(length as usize)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "chunk size overflow"))?,
        ))
    }
}
