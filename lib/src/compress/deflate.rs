use crate::entry::CompressionLevelImpl;
use crate::CompressionLevel;
use flate2::Compression;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

/// Represents a Deflate compression level.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct DeflateCompressionLevel(Compression);

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
}
