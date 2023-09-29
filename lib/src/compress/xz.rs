pub(crate) type CompressionLevel = u32;

impl From<crate::CompressionLevel> for CompressionLevel {
    #[inline]
    fn from(value: crate::CompressionLevel) -> Self {
        if value == crate::CompressionLevel::default() {
            6
        } else {
            value.0 as u32
        }
    }
}
