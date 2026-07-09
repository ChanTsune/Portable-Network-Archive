use std::str::FromStr;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub(crate) struct ChunkType(pub(crate) pna::ChunkType);

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub(crate) enum ParseChunkTypeError {
    #[error("chunk type must be exactly 4 ASCII characters")]
    InvalidLength,
    #[error("chunk type must contain only ASCII alphabetic characters")]
    NonAsciiAlphabetic,
}

impl FromStr for ChunkType {
    type Err = ParseChunkTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; 4] = s
            .as_bytes()
            .try_into()
            .map_err(|_| ParseChunkTypeError::InvalidLength)?;
        if !bytes.iter().all(u8::is_ascii_alphabetic) {
            return Err(ParseChunkTypeError::NonAsciiAlphabetic);
        }
        // SAFETY: All four bytes are ASCII alphabetic as verified above.
        Ok(Self(unsafe { pna::ChunkType::from_unchecked(bytes) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_valid_standard_chunk() {
        let chunk_type = ChunkType::from_str("FHED").unwrap();
        assert_eq!(chunk_type.0, pna::ChunkType::FHED);
    }

    #[test]
    fn from_str_valid_private_chunk() {
        let chunk_type = ChunkType::from_str("myTy").unwrap();
        assert_eq!(chunk_type.0, unsafe {
            pna::ChunkType::from_unchecked(*b"myTy")
        });
    }

    #[test]
    fn from_str_invalid_length() {
        assert_eq!(
            ChunkType::from_str("toolong").unwrap_err(),
            ParseChunkTypeError::InvalidLength
        );
        assert_eq!(
            ChunkType::from_str("abc").unwrap_err(),
            ParseChunkTypeError::InvalidLength
        );
    }

    #[test]
    fn from_str_non_ascii_alphabetic() {
        assert_eq!(
            ChunkType::from_str("FH1D").unwrap_err(),
            ParseChunkTypeError::NonAsciiAlphabetic
        );
    }
}
