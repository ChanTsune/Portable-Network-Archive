use crate::CompressionLevel;
use flate2::Compression;

impl From<CompressionLevel> for Compression {
    #[inline]
    fn from(value: CompressionLevel) -> Self {
        if value == CompressionLevel::DEFAULT {
            Self::default()
        } else {
            Self::new((value.0 as u32).clamp(Self::none().level(), Self::best().level()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
