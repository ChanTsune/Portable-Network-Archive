use pna::{ChunkType, RawChunk};

/// Private chunk type for file flags (fflags).
/// Name follows PNA chunk naming convention where case has semantic meaning:
/// - lowercase first letter: ancillary (not critical)
/// - lowercase second letter: private (not public)
/// - uppercase third letter: reserved
/// - lowercase fourth letter: safe to copy
#[allow(non_upper_case_globals)]
pub const ffLg: ChunkType = unsafe { ChunkType::from_unchecked(*b"ffLg") };

pub fn fflag_chunk(flag: &str) -> RawChunk {
    RawChunk::from_data(ffLg, flag.as_bytes())
}
