use crate::entry::{CompressionLevel, CompressionLevelImpl};
use std::{num::ParseIntError, str::FromStr};

/// Represents a XZ compression level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct XZCompressionLevel(u32);

impl XZCompressionLevel {
    /// Default compression level for xz.
    const DEFAULT: Self = Self(6);
    /// Minimum compression level for xz.
    const MIN: Self = Self(0);
    /// Minimum compression level for xz.
    const MAX: Self = Self(9);

    #[inline]
    fn new(level: u32) -> Option<Self> {
        let level = Self(level);
        if Self::MIN <= level && level <= Self::MAX {
            Some(level)
        } else {
            None
        }
    }
}

impl Default for XZCompressionLevel {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<CompressionLevel> for XZCompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        match value.0 {
            CompressionLevelImpl::Min => Self(0),
            CompressionLevelImpl::Max => Self(9),
            CompressionLevelImpl::Default => Self::DEFAULT,
            CompressionLevelImpl::Custom(value) => Self(value.clamp(0, 9) as _),
        }
    }
}

impl From<XZCompressionLevel> for CompressionLevel {
    #[inline]
    fn from(value: XZCompressionLevel) -> Self {
        CompressionLevel(CompressionLevelImpl::Custom(value.0 as _))
    }
}

impl From<XZCompressionLevel> for u32 {
    #[inline]
    fn from(value: XZCompressionLevel) -> Self {
        value.0
    }
}

impl FromStr for XZCompressionLevel {
    type Err = ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("min") {
            Ok(Self::MIN)
        } else if s.eq_ignore_ascii_case("max") {
            Ok(Self::MAX)
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
            XZCompressionLevel::from(CompressionLevel::from(0)),
            XZCompressionLevel(0)
        );
    }

    #[test]
    fn max() {
        assert_eq!(
            XZCompressionLevel::from(CompressionLevel::from(9)),
            XZCompressionLevel(9)
        );
    }

    #[test]
    fn default() {
        assert_eq!(
            XZCompressionLevel::from(CompressionLevel::default()),
            XZCompressionLevel(6)
        );
    }

    #[test]
    fn out_of_range() {
        assert_eq!(
            XZCompressionLevel::from(CompressionLevel::from(100)),
            XZCompressionLevel(9)
        );
    }

    #[test]
    fn from_str() {
        assert_eq!(
            XZCompressionLevel::from_str("default").unwrap(),
            XZCompressionLevel::new(6).unwrap()
        );
        assert_eq!(
            XZCompressionLevel::from_str("min").unwrap(),
            XZCompressionLevel::new(0).unwrap()
        );
        assert_eq!(
            XZCompressionLevel::from_str("max").unwrap(),
            XZCompressionLevel::new(9).unwrap()
        );
        assert_eq!(
            XZCompressionLevel::from_str("0").unwrap(),
            XZCompressionLevel::new(0).unwrap()
        );
        assert_eq!(
            XZCompressionLevel::from_str("9").unwrap(),
            XZCompressionLevel::new(9).unwrap()
        );
        assert!(XZCompressionLevel::from_str("10").is_err());
    }
}
