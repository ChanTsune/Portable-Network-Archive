use crate::entry::{CompressionLevel, CompressionLevelImpl};

/// Represents a XZ compression level.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct XZCompressionLevel(u32);

impl XZCompressionLevel {
    /// Default compression level for xz.
    const DEFAULT: Self = Self(6);
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
}
