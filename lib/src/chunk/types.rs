#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ChunkType(pub [u8; 4]);

// -- Critical chunks --

/// Archive header
pub const AHED: ChunkType = ChunkType(*b"AHED");
/// Archive end
pub const AEND: ChunkType = ChunkType(*b"AEND");
/// File header
pub const FHED: ChunkType = ChunkType(*b"FHED");
/// Password hash string format
pub const PHSF: ChunkType = ChunkType(*b"PHSF");
/// File data
pub const FDAT: ChunkType = ChunkType(*b"FDAT");
/// File end
pub const FEND: ChunkType = ChunkType(*b"FEND");
