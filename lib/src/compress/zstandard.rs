pub(crate) type CompressionLevel = i32;

impl From<crate::CompressionLevel> for CompressionLevel {
    #[inline]
    fn from(value: crate::CompressionLevel) -> Self {
        if value == crate::CompressionLevel::default() {
            0
        } else {
            value.0 as CompressionLevel
        }
    }
}
