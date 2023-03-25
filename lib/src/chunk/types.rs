use std::fmt::{self, Display, Formatter};

/// A 4-byte chunk type code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ChunkType(pub(crate) [u8; 4]);

impl ChunkType {
    // -- Critical chunks --
    /// Archive header
    pub const AHED: ChunkType = ChunkType(*b"AHED");
    /// Archive end
    pub const AEND: ChunkType = ChunkType(*b"AEND");
    /// Archive next part marker
    pub const ANXT: ChunkType = ChunkType(*b"ANXT");
    /// File header
    pub const FHED: ChunkType = ChunkType(*b"FHED");
    /// Password hash string format
    pub const PHSF: ChunkType = ChunkType(*b"PHSF");
    /// File data
    pub const FDAT: ChunkType = ChunkType(*b"FDAT");
    /// File end
    pub const FEND: ChunkType = ChunkType(*b"FEND");

    // -- Auxiliary chunks --
    /// Creation datetime
    #[allow(non_upper_case_globals)]
    pub const cTIM: ChunkType = ChunkType(*b"cTIM");
    /// Last modified datetime
    #[allow(non_upper_case_globals)]
    pub const mTIM: ChunkType = ChunkType(*b"mTIM");
    /// File permissions
    #[allow(non_upper_case_globals)]
    pub const fPRM: ChunkType = ChunkType(*b"pPRM");
}

impl Display for ChunkType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(unsafe { std::str::from_utf8_unchecked(&self.0) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_string() {
        assert_eq!("AHED", ChunkType::AHED.to_string());
    }
}
