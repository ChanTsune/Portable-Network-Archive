pub(crate) type CompressionLevel = u32;

impl From<crate::CompressionLevel> for CompressionLevel {
    #[inline]
    fn from(value: crate::CompressionLevel) -> Self {
        if value == crate::CompressionLevel::DEFAULT {
            6
        } else {
            (value.0 as Self).clamp(0, 9)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn min() {
        assert_eq!(CompressionLevel::from(crate::CompressionLevel::from(0)), 0);
    }

    #[test]
    fn max() {
        assert_eq!(CompressionLevel::from(crate::CompressionLevel::from(9)), 9);
    }

    #[test]
    fn default() {
        assert_eq!(
            CompressionLevel::from(crate::CompressionLevel::default()),
            6
        );
    }

    #[test]
    fn out_of_range() {
        assert_eq!(
            CompressionLevel::from(crate::CompressionLevel::from(100)),
            9
        );
    }
}
