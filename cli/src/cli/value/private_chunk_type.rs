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
