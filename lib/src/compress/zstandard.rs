use crate::{entry::CompressionLevelImpl, CompressionLevel};
use std::{num::ParseIntError, str::FromStr};
use zstd::zstd_safe;

/// Represents a Zstd compression level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ZstdCompressionLevel(zstd_safe::CompressionLevel);

impl ZstdCompressionLevel {
    /// Default compression level for zstd.
    const DEFAULT: Self = Self(zstd::DEFAULT_COMPRESSION_LEVEL);

    #[inline]
    fn min() -> Self {
        Self(zstd_safe::min_c_level())
    }

    #[inline]
    fn max() -> Self {
        Self(zstd_safe::max_c_level())
    }

    #[inline]
    fn new(level: zstd_safe::CompressionLevel) -> Option<Self> {
        let level = Self(level);
        if Self::min() <= level && level <= Self::max() {
            Some(level)
        } else {
            None
        }
    }
}

impl Default for ZstdCompressionLevel {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<CompressionLevel> for ZstdCompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        match value.0 {
            CompressionLevelImpl::Min => Self::min(),
            CompressionLevelImpl::Max => Self::max(),
            CompressionLevelImpl::Default => Self::DEFAULT,
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

impl FromStr for ZstdCompressionLevel {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("min") {
            Ok(Self::min())
        } else if s.eq_ignore_ascii_case("max") {
            Ok(Self::max())
        } else if s.eq_ignore_ascii_case("default") {
            Ok(Self::default())
        } else {
            Self::new(s.parse()?).ok_or_else(||
                // NOTE: Hack generate `ParseIntError`.
                u8::from_str_radix("999", 2).unwrap_err())
        }
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

    #[test]
    fn from_str() {
        assert_eq!(
            ZstdCompressionLevel::from_str("default").unwrap(),
            ZstdCompressionLevel::default()
        );
        assert_eq!(
            ZstdCompressionLevel::from_str("min").unwrap(),
            ZstdCompressionLevel::min()
        );
        assert_eq!(
            ZstdCompressionLevel::from_str("max").unwrap(),
            ZstdCompressionLevel::max()
        );
        assert_eq!(
            ZstdCompressionLevel::from_str("5").unwrap(),
            ZstdCompressionLevel::new(5).unwrap()
        );
    }
}
