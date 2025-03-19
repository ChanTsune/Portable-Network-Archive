use crate::entry::CompressionLevelImpl;
use crate::CompressionLevel;
use zstd::zstd_safe;

/// Represents a Zstd compression level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ZstdCompressionLevel(zstd_safe::CompressionLevel);

impl From<CompressionLevel> for ZstdCompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        match value.0 {
            CompressionLevelImpl::Min => Self(zstd_safe::min_c_level()),
            CompressionLevelImpl::Max => Self(zstd_safe::max_c_level()),
            CompressionLevelImpl::Default => Self(zstd::DEFAULT_COMPRESSION_LEVEL),
            CompressionLevelImpl::Custom(value) => Self(
                (value as zstd_safe::CompressionLevel)
                    .clamp(zstd_safe::min_c_level(), zstd_safe::max_c_level()),
            ),
        }
    }
}

impl From<ZstdCompressionLevel> for CompressionLevel {
    #[inline]
    fn from(value: ZstdCompressionLevel) -> Self {
        Self(CompressionLevelImpl::Custom(value.0 as _))
    }
}

impl From<ZstdCompressionLevel> for zstd_safe::CompressionLevel {
    #[inline]
    fn from(value: ZstdCompressionLevel) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn min() {
        assert_eq!(
            ZstdCompressionLevel::from(CompressionLevel::from(0)),
            ZstdCompressionLevel(0)
        );
    }

    #[test]
    fn max() {
        assert_eq!(
            ZstdCompressionLevel::from(CompressionLevel::from(22)),
            ZstdCompressionLevel(zstd_safe::max_c_level())
        );
    }

    #[test]
    fn default() {
        assert_eq!(
            ZstdCompressionLevel::from(CompressionLevel::default()),
            ZstdCompressionLevel(zstd::DEFAULT_COMPRESSION_LEVEL)
        );
    }

    #[test]
    fn out_of_range() {
        assert_eq!(
            ZstdCompressionLevel::from(CompressionLevel::from(100)),
            ZstdCompressionLevel(zstd_safe::max_c_level())
        );
    }
}
