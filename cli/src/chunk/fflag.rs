use pna::{ChunkType, RawChunk};

pub const ffLg: ChunkType = unsafe { ChunkType::from_unchecked(*b"ffLg") };

pub fn fflag_chunk(flag: &str) -> RawChunk {
    RawChunk::from_data(ffLg, flag.as_bytes())
}
