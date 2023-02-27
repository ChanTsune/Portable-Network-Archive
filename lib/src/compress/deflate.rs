pub(crate) type CompressionLevel = flate2::Compression;

impl From<crate::CompressionLevel> for CompressionLevel {
    fn from(value: crate::CompressionLevel) -> Self {
        if value == crate::CompressionLevel::default() {
            flate2::Compression::default()
        } else {
            flate2::Compression::new(value.0 as u32)
        }
    }
}
