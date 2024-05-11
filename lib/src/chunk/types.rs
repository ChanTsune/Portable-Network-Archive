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
    /// Raw file size
    #[allow(non_upper_case_globals)]
    pub const fSIZ: ChunkType = ChunkType(*b"fSIZ");
    /// Creation datetime
    #[allow(non_upper_case_globals)]
    pub const cTIM: ChunkType = ChunkType(*b"cTIM");
    /// Last modified datetime
    #[allow(non_upper_case_globals)]
    pub const mTIM: ChunkType = ChunkType(*b"mTIM");
    /// Last accessed datetime
    #[allow(non_upper_case_globals)]
    pub const aTIM: ChunkType = ChunkType(*b"aTIM");
    /// Entry permissions
    #[allow(non_upper_case_globals)]
    pub const fPRM: ChunkType = ChunkType(*b"fPRM");
    /// Extended attribute
    #[allow(non_upper_case_globals)]
    pub const xATR: ChunkType = ChunkType(*b"xATR");

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
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Creates custom [ChunkType] without any check.
    ///
    /// # Panic
    /// Printing ChunkType that contains non-utf8 characters will be panicked.
    /// ```no_run
    /// # use libpna::ChunkType;
    ///
    /// let custom_chunk_type = unsafe { ChunkType::from_unchecked([0xe3, 0x81, 0x82, 0xe3]) };
    /// format!("{}", custom_chunk_type);
    /// ```
    ///
    /// # Safety
    /// Safe when value consists only of ascii alphabetic characters ('a'...'z' and 'A'...'Z').
    /// ```
    /// # use libpna::ChunkType;
    ///
    /// let custom_chunk_type = unsafe { ChunkType::from_unchecked(*b"myTy") };
    /// format!("{}", custom_chunk_type);
    /// ```
    #[inline]
    pub const unsafe fn from_unchecked(ty: [u8; 4]) -> Self {
        Self(ty)
    }
}

impl Display for ChunkType {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(unsafe { std::str::from_utf8_unchecked(&self.0) }, f)
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
