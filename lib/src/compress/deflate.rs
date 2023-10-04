pub(crate) type CompressionLevel = flate2::Compression;

impl From<crate::CompressionLevel> for CompressionLevel {
    #[inline]
    fn from(value: crate::CompressionLevel) -> Self {
        if value == crate::CompressionLevel::default() {
            Self::default()
        } else {
            Self::new(value.0 as u32)
        }
    }
}
