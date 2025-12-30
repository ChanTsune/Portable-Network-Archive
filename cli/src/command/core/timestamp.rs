use std::time::SystemTime;

/// How to determine a single timestamp value.
///
/// This type encapsulates the resolution logic for one timestamp field
/// (mtime, ctime, or atime), supporting pass-through, override, and clamping.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum TimeSource {
    /// Use the source timestamp as-is.
    FromSource,
    /// Override with a specific value, ignoring source.
    Override(SystemTime),
    /// Clamp: use minimum of source and this value ("no newer than").
    ClampTo(SystemTime),
}

impl TimeSource {
    /// Resolve the final timestamp given a source value.
    #[must_use]
    pub(crate) fn resolve(&self, source: Option<SystemTime>) -> Option<SystemTime> {
        match self {
            Self::FromSource => source,
            Self::Override(t) => Some(*t),
            Self::ClampTo(t) => source.map(|s| (*t).min(s)),
        }
    }
}

/// Top-level timestamp handling strategy.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum TimestampStrategy {
    /// Don't preserve timestamps.
    NoPreserve,
    /// Preserve timestamps with per-field configuration.
    Preserve {
        mtime: TimeSource,
        ctime: TimeSource,
        atime: TimeSource,
    },
}

impl TimestampStrategy {
    /// Preserve all source timestamps as-is.
    #[must_use]
    pub(crate) const fn preserve() -> Self {
        Self::Preserve {
            mtime: TimeSource::FromSource,
            ctime: TimeSource::FromSource,
            atime: TimeSource::FromSource,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn time(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn time_source_from_source() {
        let source = TimeSource::FromSource;
        assert_eq!(source.resolve(Some(time(100))), Some(time(100)));
        assert_eq!(source.resolve(None), None);
    }

    #[test]
    fn time_source_override() {
        let source = TimeSource::Override(time(50));
        assert_eq!(source.resolve(Some(time(100))), Some(time(50)));
        assert_eq!(source.resolve(None), Some(time(50)));
    }

    #[test]
    fn time_source_clamp_to() {
        let source = TimeSource::ClampTo(time(50));
        // Source is newer than clamp -> use clamp
        assert_eq!(source.resolve(Some(time(100))), Some(time(50)));
        // Source is older than clamp -> use source
        assert_eq!(source.resolve(Some(time(30))), Some(time(30)));
        // No source -> None
        assert_eq!(source.resolve(None), None);
    }
}
