use crate::CompressionLevel;
use flate2::Compression;

impl From<CompressionLevel> for Compression {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        if value == CompressionLevel::DEFAULT {
            Self::default()
        } else {
            Self::new(value.0 as u32)
        }
    }
}
