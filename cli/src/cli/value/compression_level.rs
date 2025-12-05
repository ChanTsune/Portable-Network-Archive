use std::{num::ParseIntError, str::FromStr};
use thiserror::Error;

/// Error type for parsing compression level input.
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub(crate) enum ParseCompressionLevelError {
    #[error("{0}")]
    ParseInt(#[from] ParseIntError),
    #[error("{value} is not in {min}..={max}")]
    OutOfRange { value: u8, min: u8, max: u8 },
}

/// Compression level input that supports numeric values and keywords like "min" and "max".
/// The generic parameters `MIN` and `MAX` define the valid numeric range.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum CompressionLevelInput<const MIN: u8, const MAX: u8> {
    /// Minimum compression level.
    Min,
    /// Maximum compression level.
    Max,
    /// Custom numeric compression level.
    Numeric(u8),
}

impl<const MIN: u8, const MAX: u8> From<CompressionLevelInput<MIN, MAX>> for pna::CompressionLevel {
    #[inline]
    fn from(value: CompressionLevelInput<MIN, MAX>) -> Self {
        match value {
            CompressionLevelInput::Min => Self::min(),
            CompressionLevelInput::Max => Self::max(),
            CompressionLevelInput::Numeric(n) => Self::from(n),
        }
    }
}

impl<const MIN: u8, const MAX: u8> FromStr for CompressionLevelInput<MIN, MAX> {
    type Err = ParseCompressionLevelError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("min") {
            Ok(Self::Min)
        } else if s.eq_ignore_ascii_case("max") {
            Ok(Self::Max)
        } else {
            let n = s.parse::<u8>()?;
            if MIN <= n && n <= MAX {
                Ok(Self::Numeric(n))
            } else {
                Err(ParseCompressionLevelError::OutOfRange {
                    value: n,
                    min: MIN,
                    max: MAX,
                })
            }
        }
    }
}

/// Deflate compression level (1-9, min, max).
pub(crate) type DeflateLevel = CompressionLevelInput<1, 9>;

/// Zstd compression level (1-21, min, max).
pub(crate) type ZstdLevel = CompressionLevelInput<1, 21>;

/// Xz compression level (0-9, min, max).
pub(crate) type XzLevel = CompressionLevelInput<0, 9>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_min() {
        assert_eq!(DeflateLevel::from_str("min").unwrap(), DeflateLevel::Min);
        assert_eq!(ZstdLevel::from_str("MIN").unwrap(), ZstdLevel::Min);
        assert_eq!(XzLevel::from_str("Min").unwrap(), XzLevel::Min);
    }

    #[test]
    fn parse_max() {
        assert_eq!(DeflateLevel::from_str("max").unwrap(), DeflateLevel::Max);
        assert_eq!(ZstdLevel::from_str("MAX").unwrap(), ZstdLevel::Max);
        assert_eq!(XzLevel::from_str("Max").unwrap(), XzLevel::Max);
    }

    #[test]
    fn deflate_range() {
        assert!(DeflateLevel::from_str("0").is_err());
        assert_eq!(
            DeflateLevel::from_str("1").unwrap(),
            DeflateLevel::Numeric(1)
        );
        assert_eq!(
            DeflateLevel::from_str("9").unwrap(),
            DeflateLevel::Numeric(9)
        );
        assert!(DeflateLevel::from_str("10").is_err());
    }

    #[test]
    fn zstd_range() {
        assert!(ZstdLevel::from_str("0").is_err());
        assert_eq!(ZstdLevel::from_str("1").unwrap(), ZstdLevel::Numeric(1));
        assert_eq!(ZstdLevel::from_str("21").unwrap(), ZstdLevel::Numeric(21));
        assert!(ZstdLevel::from_str("22").is_err());
    }

    #[test]
    fn xz_range() {
        assert_eq!(XzLevel::from_str("0").unwrap(), XzLevel::Numeric(0));
        assert_eq!(XzLevel::from_str("9").unwrap(), XzLevel::Numeric(9));
        assert!(XzLevel::from_str("10").is_err());
    }

    #[test]
    fn parse_invalid() {
        assert!(DeflateLevel::from_str("invalid").is_err());
        assert!(ZstdLevel::from_str("-1").is_err());
        assert!(XzLevel::from_str("256").is_err());
    }
}
