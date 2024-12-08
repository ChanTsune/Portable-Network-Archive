use crate::entry::{CompressionLevel, CompressionLevelImpl};

pub type XZCompressionLevel = u32;

impl From<CompressionLevel> for XZCompressionLevel {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        match value.0 {
            CompressionLevelImpl::Min => 0,
            CompressionLevelImpl::Max => 9,
            CompressionLevelImpl::Default => 6,
            CompressionLevelImpl::Custom(value) => (value as Self).clamp(0, 9),
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
        assert_eq!(XZCompressionLevel::from(CompressionLevel::from(0)), 0);
    }

    #[test]
    fn max() {
        assert_eq!(XZCompressionLevel::from(CompressionLevel::from(9)), 9);
    }

    #[test]
    fn default() {
        assert_eq!(XZCompressionLevel::from(CompressionLevel::default()), 6);
    }

    #[test]
    fn out_of_range() {
        assert_eq!(XZCompressionLevel::from(CompressionLevel::from(100)), 9);
    }
}
