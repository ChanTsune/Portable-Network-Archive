use pna::{ChunkType, ChunkTypeError};
use std::str::FromStr;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct PrivateChunkType(pub(crate) ChunkType);

impl FromStr for PrivateChunkType {
    type Err = ChunkTypeError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(ChunkType::private(
            s.as_bytes()
                .try_into()
                .map_err(|_| ChunkTypeError::NonPrivateChunkType)?,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_valid() {
        let chunk_type = PrivateChunkType::from_str("myTy").unwrap();
        assert_eq!(chunk_type.0, ChunkType::private(*b"myTy").unwrap());
    }

    #[test]
    fn from_str_invalid_length() {
        assert_eq!(
            PrivateChunkType::from_str("invalid").unwrap_err(),
            ChunkTypeError::NonPrivateChunkType
        );
    }

    #[test]
    fn from_str_invalid_second_char() {
        assert_eq!(
            PrivateChunkType::from_str("pRIv").unwrap_err(),
            ChunkTypeError::NonPrivateChunkType
        );
    }

    #[test]
    fn from_str_invalid_third_char() {
        assert_eq!(
            PrivateChunkType::from_str("rese").unwrap_err(),
            ChunkTypeError::Reserved
        );
    }

    #[test]
    fn from_str_non_ascii() {
        assert_eq!(
            PrivateChunkType::from_str("zeR\0").unwrap_err(),
            ChunkTypeError::NonAsciiAlphabetic
        );
    }
}
