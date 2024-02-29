use crate::CompressionLevel;
use zstd::zstd_safe;

impl From<CompressionLevel> for zstd_safe::CompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        if value == CompressionLevel::DEFAULT {
            0
        } else {
            value.0 as Self
        }
    }
}
