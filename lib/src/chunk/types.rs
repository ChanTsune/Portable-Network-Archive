use std::fmt::{self, Display, Formatter};

/// A 4-byte chunk type code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ChunkType(pub(crate) [u8; 4]);

impl ChunkType {
    // -- Critical chunks --
    /// Archive header
    pub const AHED: ChunkType = ChunkType(*b"AHED");
    /// Archive end marker
    pub const AEND: ChunkType = ChunkType(*b"AEND");
    /// Archive next part marker
    pub const ANXT: ChunkType = ChunkType(*b"ANXT");
    /// Entry header
    pub const FHED: ChunkType = ChunkType(*b"FHED");
    /// Password hash string format
    pub const PHSF: ChunkType = ChunkType(*b"PHSF");
    /// Entry data stream
    pub const FDAT: ChunkType = ChunkType(*b"FDAT");
    /// Entry data stream end marker
    pub const FEND: ChunkType = ChunkType(*b"FEND");
    /// Solid mode data header
    pub const SHED: ChunkType = ChunkType(*b"SHED");
    /// Solid mode data stream
    pub const SDAT: ChunkType = ChunkType(*b"SDAT");
    /// Solid mode data stream end marker
    pub const SEND: ChunkType = ChunkType(*b"SEND");

    // -- Auxiliary chunks --
    /// Creation datetime
    #[allow(non_upper_case_globals)]
    pub const cTIM: ChunkType = ChunkType(*b"cTIM");
    /// Last modified datetime
    #[allow(non_upper_case_globals)]
    pub const mTIM: ChunkType = ChunkType(*b"mTIM");
    /// Entry permissions
    #[allow(non_upper_case_globals)]
    pub const fPRM: ChunkType = ChunkType(*b"fPRM");

    /// Returns the length of the chunk type code.
    ///
    /// # Returns
    ///
    /// An integer value representing the length of the chunk type code.
    ///
    /// # Example
    ///
    /// ```
    /// use libpna::ChunkType;
    ///
    /// let chunk_type = ChunkType::AHED;
    ///
    /// assert_eq!(chunk_type.len(), 4);
    /// ```
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        self.0.len()
    }
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
