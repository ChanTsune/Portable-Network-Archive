use crate::entry::CompressionLevelImpl;
use crate::CompressionLevel;
use zstd::zstd_safe;

impl From<CompressionLevel> for zstd_safe::CompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        match value.0 {
            CompressionLevelImpl::Min => zstd_safe::min_c_level(),
            CompressionLevelImpl::Max => zstd_safe::max_c_level(),
            CompressionLevelImpl::Default => zstd::DEFAULT_COMPRESSION_LEVEL,
            CompressionLevelImpl::Custom(value) => {
                (value as Self).clamp(zstd_safe::min_c_level(), zstd_safe::max_c_level())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn min() {
        assert_eq!(
            zstd_safe::CompressionLevel::from(CompressionLevel::from(0)),
            0
        );
    }

    #[test]
    fn max() {
        assert_eq!(
            zstd_safe::CompressionLevel::from(CompressionLevel::from(22)),
            zstd_safe::max_c_level()
        );
    }

    #[test]
    fn default() {
        assert_eq!(
            zstd_safe::CompressionLevel::from(CompressionLevel::default()),
            zstd::DEFAULT_COMPRESSION_LEVEL
        );
    }

    #[test]
    fn out_of_range() {
        assert_eq!(
            zstd_safe::CompressionLevel::from(CompressionLevel::from(100)),
            zstd_safe::max_c_level()
        );
    }
}
