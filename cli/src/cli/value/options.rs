use super::{DeflateLevel, XzLevel, ZstdLevel};
use std::str::FromStr;

/// Archive options parsed from `--options` argument.
///
/// Supports bsdtar-compatible key=value and module:key=value format.
#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub(crate) struct ArchiveOptions {
    /// Global compression level (applies to any algorithm).
    pub(crate) compression_level: Option<pna::CompressionLevel>,
    /// Deflate-specific compression level.
    pub(crate) deflate_compression_level: Option<DeflateLevel>,
    /// Zstd-specific compression level.
    pub(crate) zstd_compression_level: Option<ZstdLevel>,
    /// Xz-specific compression level.
    pub(crate) xz_compression_level: Option<XzLevel>,
}

fn parse_global_compression_level(s: &str) -> Result<pna::CompressionLevel, String> {
    if s.eq_ignore_ascii_case("min") {
        Ok(pna::CompressionLevel::min())
    } else if s.eq_ignore_ascii_case("max") {
        Ok(pna::CompressionLevel::max())
    } else {
        s.parse::<u8>()
            .map(pna::CompressionLevel::from)
            .map_err(|e| e.to_string())
    }
}

impl FromStr for ArchiveOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self::default();

        for param in s.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }

            let Some((left, value)) = param.split_once('=') else {
                return Err(format!(
                    "Invalid option format `{param}`, expected key=value"
                ));
            };

            if let Some((module, key)) = left.split_once(':') {
                match (module, key) {
                    ("deflate", "compression-level") => {
                        result.deflate_compression_level = Some(
                            value
                                .parse()
                                .map_err(|e| format!("deflate:compression-level: {e}"))?,
                        );
                    }
                    ("zstd", "compression-level") => {
                        result.zstd_compression_level = Some(
                            value
                                .parse()
                                .map_err(|e| format!("zstd:compression-level: {e}"))?,
                        );
                    }
                    ("xz", "compression-level") => {
                        result.xz_compression_level = Some(
                            value
                                .parse()
                                .map_err(|e| format!("xz:compression-level: {e}"))?,
                        );
                    }
                    (module, key) => {
                        return Err(format!("Unknown option `{module}:{key}`"));
                    }
                }
            } else {
                match left {
                    "compression-level" => {
                        result.compression_level = Some(parse_global_compression_level(value)?);
                    }
                    key => {
                        return Err(format!("Unknown option `{key}`"));
                    }
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_compression_level() {
        let opts = ArchiveOptions::from_str("compression-level=9").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::from(9)));
        assert_eq!(opts.zstd_compression_level, None);
    }

    #[test]
    fn parse_module_specific_compression_level() {
        let opts = ArchiveOptions::from_str("zstd:compression-level=15").unwrap();
        assert_eq!(opts.compression_level, None);
        assert_eq!(
            opts.zstd_compression_level,
            Some(ZstdLevel::Numeric(15.try_into().unwrap()))
        );
    }

    #[test]
    fn parse_multiple_options() {
        let opts =
            ArchiveOptions::from_str("compression-level=5,zstd:compression-level=10").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::from(5)));
        assert_eq!(
            opts.zstd_compression_level,
            Some(ZstdLevel::Numeric(10.try_into().unwrap()))
        );
    }

    #[test]
    fn parse_min_max_keywords() {
        let opts = ArchiveOptions::from_str("compression-level=min").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::min()));

        let opts = ArchiveOptions::from_str("xz:compression-level=MAX").unwrap();
        assert_eq!(opts.xz_compression_level, Some(XzLevel::Max));
    }

    #[test]
    fn parse_all_modules() {
        let opts = ArchiveOptions::from_str(
            "deflate:compression-level=6,zstd:compression-level=15,xz:compression-level=9",
        )
        .unwrap();
        assert_eq!(
            opts.deflate_compression_level,
            Some(DeflateLevel::Numeric(6.try_into().unwrap()))
        );
        assert_eq!(
            opts.zstd_compression_level,
            Some(ZstdLevel::Numeric(15.try_into().unwrap()))
        );
        assert_eq!(
            opts.xz_compression_level,
            Some(XzLevel::Numeric(9.try_into().unwrap()))
        );
    }

    #[test]
    fn parse_empty_string() {
        let opts = ArchiveOptions::from_str("").unwrap();
        assert_eq!(opts, ArchiveOptions::default());
    }

    #[test]
    fn parse_whitespace_handling() {
        let opts =
            ArchiveOptions::from_str(" compression-level=9 , zstd:compression-level=15 ").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::from(9)));
        assert_eq!(
            opts.zstd_compression_level,
            Some(ZstdLevel::Numeric(15.try_into().unwrap()))
        );
    }

    #[test]
    fn error_unknown_option() {
        assert!(ArchiveOptions::from_str("unknown=value").is_err());
    }

    #[test]
    fn error_unknown_module() {
        assert!(ArchiveOptions::from_str("lzma:compression-level=5").is_err());
    }

    #[test]
    fn error_unknown_module_key() {
        assert!(ArchiveOptions::from_str("zstd:unknown=5").is_err());
    }

    #[test]
    fn error_invalid_format_no_value() {
        assert!(ArchiveOptions::from_str("compression-level").is_err());
    }

    #[test]
    fn error_invalid_level_value() {
        assert!(ArchiveOptions::from_str("compression-level=abc").is_err());
    }

    #[test]
    fn parse_global_compression_level_max() {
        let opts = ArchiveOptions::from_str("compression-level=max").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::max()));

        let opts = ArchiveOptions::from_str("compression-level=MAX").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::max()));
    }

    #[test]
    fn module_prefix_routes_to_correct_field() {
        let opts = ArchiveOptions::from_str("deflate:compression-level=5").unwrap();
        assert!(opts.deflate_compression_level.is_some());
        assert!(opts.zstd_compression_level.is_none());
        assert!(opts.xz_compression_level.is_none());

        let opts = ArchiveOptions::from_str("zstd:compression-level=10").unwrap();
        assert!(opts.deflate_compression_level.is_none());
        assert!(opts.zstd_compression_level.is_some());
        assert!(opts.xz_compression_level.is_none());
    }

    #[test]
    fn error_propagates_with_module_context() {
        let err = ArchiveOptions::from_str("deflate:compression-level=invalid").unwrap_err();
        assert!(err.contains("deflate:compression-level:"));
    }

    #[test]
    fn duplicate_option_last_wins() {
        let opts = ArchiveOptions::from_str("compression-level=5,compression-level=9").unwrap();
        assert_eq!(opts.compression_level, Some(pna::CompressionLevel::from(9)));
    }
}
