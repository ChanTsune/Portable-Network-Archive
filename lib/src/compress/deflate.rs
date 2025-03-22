use crate::{entry::CompressionLevelImpl, CompressionLevel};
use flate2::Compression;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    num::ParseIntError,
    str::FromStr,
};

/// Represents a Deflate compression level.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct DeflateCompressionLevel(Compression);

impl DeflateCompressionLevel {
    /// Minimum compression level for deflate.
    const MIN: Self = Self(Compression::none());
    /// Maximum compression level for deflate.
    const MAX: Self = Self(Compression::best());

    #[inline]
    fn new(level: u32) -> Option<Self> {
        let level = Self(Compression::new(level));
        if Self::MIN <= level && level <= Self::MAX {
            Some(level)
        } else {
            None
        }
    }
}

impl Default for DeflateCompressionLevel {
    #[inline]
    fn default() -> Self {
        Self(Compression::default())
    }
}

impl From<DeflateCompressionLevel> for i64 {
    #[inline]
    fn from(value: DeflateCompressionLevel) -> Self {
        value.0.level().into()
    }
}

impl From<Compression> for DeflateCompressionLevel {
    #[inline]
    fn from(value: Compression) -> Self {
        Self(value)
    }
}

impl From<DeflateCompressionLevel> for Compression {
    #[inline]
    fn from(value: DeflateCompressionLevel) -> Self {
        value.0
    }
}

impl From<CompressionLevel> for DeflateCompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        Self(value.into())
    }
}

impl Hash for DeflateCompressionLevel {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.level().hash(state)
    }
}

impl Ord for DeflateCompressionLevel {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.level().cmp(&other.0.level())
    }
}

impl PartialOrd for DeflateCompressionLevel {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<CompressionLevel> for Compression {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        match value.0 {
            CompressionLevelImpl::Min => Self::none(),
            CompressionLevelImpl::Max => Self::best(),
            CompressionLevelImpl::Default => Self::default(),
            CompressionLevelImpl::Custom(value) => {
                Self::new((value as u32).clamp(Self::none().level(), Self::best().level()))
            }
        }
    }
}

impl FromStr for DeflateCompressionLevel {
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
mod test {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn min() {
        assert_eq!(
            Compression::from(CompressionLevel::from(0)),
            Compression::none()
        );
    }

    #[test]
    fn max() {
        assert_eq!(
            Compression::from(CompressionLevel::from(9)),
            Compression::best()
        );
    }

    #[test]
    fn default() {
        assert_eq!(
            Compression::from(CompressionLevel::default()),
            Compression::default()
        );
    }

    #[test]
    fn out_of_range() {
        assert_eq!(
            Compression::from(CompressionLevel::from(100)),
            Compression::best()
        );
    }

    #[test]
    fn from_str() {
        assert_eq!(
            DeflateCompressionLevel::from_str("default").unwrap(),
            DeflateCompressionLevel::new(6).unwrap()
        );
        assert_eq!(
            DeflateCompressionLevel::from_str("min").unwrap(),
            DeflateCompressionLevel::new(0).unwrap()
        );
        assert_eq!(
            DeflateCompressionLevel::from_str("max").unwrap(),
            DeflateCompressionLevel::new(9).unwrap()
        );
        assert_eq!(
            DeflateCompressionLevel::from_str("5").unwrap(),
            DeflateCompressionLevel::new(5).unwrap()
        );
        assert_eq!(
            DeflateCompressionLevel::from_str("0").unwrap(),
            DeflateCompressionLevel::new(0).unwrap()
        );
        assert_eq!(
            DeflateCompressionLevel::from_str("9").unwrap(),
            DeflateCompressionLevel::new(9).unwrap()
        );
        assert!(DeflateCompressionLevel::from_str("10").is_err());
    }
}
