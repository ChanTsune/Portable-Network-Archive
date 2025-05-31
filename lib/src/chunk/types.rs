use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

/// [ChunkType] validation error.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ChunkTypeError {
    /// Contains non-ascii alphabet error.
    NonAsciiAlphabetic,
    /// The second character is not lowercase error.
    NonPrivateChunkType,
    /// The third character is not uppercase error.
    Reserved,
}

impl Display for ChunkTypeError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(
            match self {
                Self::NonAsciiAlphabetic => "All characters are must be ASCII alphabet",
                Self::NonPrivateChunkType => "The second character is must be lowercase",
                Self::Reserved => "The third character is must be uppercase",
            },
            f,
        )
    }
}

impl Error for ChunkTypeError {}

/// A 4-byte chunk type code.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
    /// Nanoseconds for creation datetime
    #[allow(non_upper_case_globals)]
    pub const cTNS: ChunkType = ChunkType(*b"cTNS");
    /// Nanoseconds for last modified datetime
    #[allow(non_upper_case_globals)]
    pub const mTNS: ChunkType = ChunkType(*b"mTNS");
    /// Nanoseconds for last accessed datetime
    #[allow(non_upper_case_globals)]
    pub const aTNS: ChunkType = ChunkType(*b"aTNS");
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

    /// Creates private [ChunkType].
    ///
    /// # Errors
    ///
    /// This function will return an error in the following cases:
    /// - Value contains non-ASCII alphabet characters
    /// - The second character is not lowercase
    /// - The third character is not uppercase
    ///
    /// # Examples
    ///
    /// ```
    /// # use libpna::{ChunkType, ChunkTypeError};
    /// assert!(ChunkType::private(*b"myTy").is_ok());
    /// assert_eq!(
    ///     ChunkType::private(*b"zeR\0").unwrap_err(),
    ///     ChunkTypeError::NonAsciiAlphabetic
    /// );
    /// assert_eq!(
    ///     ChunkType::private(*b"pRIv").unwrap_err(),
    ///     ChunkTypeError::NonPrivateChunkType
    /// );
    /// assert_eq!(
    ///     ChunkType::private(*b"rese").unwrap_err(),
    ///     ChunkTypeError::Reserved
    /// );
    /// ```
    #[inline]
    pub const fn private(ty: [u8; 4]) -> Result<Self, ChunkTypeError> {
        // NOTE: use a while statement for const context.
        let mut idx = 0;
        while idx < ty.len() {
            if !ty[idx].is_ascii_alphabetic() {
                return Err(ChunkTypeError::NonAsciiAlphabetic);
            }
            idx += 1;
        }
        if !ty[1].is_ascii_lowercase() {
            return Err(ChunkTypeError::NonPrivateChunkType);
        }
        if !ty[2].is_ascii_uppercase() {
            return Err(ChunkTypeError::Reserved);
        }
        Ok(Self(ty))
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

    // -- Chunk type determination --

    /// Returns true if the chunk is critical.
    #[inline]
    pub const fn is_critical(&self) -> bool {
        self.0[0] & 32 == 0
    }

    /// Returns true if the chunk is private.
    #[inline]
    pub const fn is_private(&self) -> bool {
        self.0[1] & 32 != 0
    }

    /// Checks whether the reserved bit of the chunk name is set.
    /// If it is set, the chunk name is invalid.
    #[inline]
    pub const fn is_set_reserved(&self) -> bool {
        self.0[2] & 32 != 0
    }

    /// Returns true if the chunk is safe to copy if unknown.
    #[inline]
    pub const fn is_safe_to_copy(&self) -> bool {
        self.0[3] & 32 != 0
    }
}

impl Debug for ChunkType {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        struct DebugType([u8; 4]);

        impl Debug for DebugType {
            #[inline]
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                for &c in &self.0[..] {
                    write!(f, "{}", char::from(c).escape_debug())?;
                }
                Ok(())
            }
        }

        f.debug_struct("ChunkType")
            .field("type", &DebugType(self.0))
            .field("critical", &self.is_critical())
            .field("private", &self.is_private())
            .field("reserved", &self.is_set_reserved())
            .field("safe_to_copy", &self.is_safe_to_copy())
            .finish()
    }
}

impl Display for ChunkType {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // SAFETY: A field checked to be ASCII alphabetic in the constructor.
        Display::fmt(unsafe { std::str::from_utf8_unchecked(&self.0) }, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn to_string() {
        assert_eq!("AHED", ChunkType::AHED.to_string());
    }

    #[test]
    fn is_critical() {
        assert!(ChunkType::AHED.is_critical());
        assert!(!ChunkType::cTIM.is_critical());
    }

    #[test]
    fn is_private() {
        assert!(!ChunkType::AHED.is_private());
        assert!(ChunkType::private(*b"myTy").unwrap().is_private());
    }

    #[test]
    fn is_set_reserved() {
        assert!(!ChunkType::AHED.is_set_reserved());
    }

    #[test]
    fn is_safe_to_copy() {
        assert!(!ChunkType::AHED.is_safe_to_copy());
    }
}
