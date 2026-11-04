use pna::Duration;
use std::{cmp::Ordering, time::SystemTime};

/// Compares two timestamps at the precision `archived` was stored with, so that an
/// entry carrying no sub-second precision compares by whole seconds only.
pub(crate) fn cmp_at_stored_precision(archived: Duration, fs: Duration) -> Ordering {
    if archived.subsec_nanoseconds() == 0 {
        floor_seconds(archived).cmp(&floor_seconds(fs))
    } else {
        archived.cmp(&fs)
    }
}

/// A whole second denotes the interval starting at it on both sides of the epoch;
/// [`Duration::whole_seconds`] alone truncates towards zero.
fn floor_seconds(duration: Duration) -> i64 {
    let seconds = duration.whole_seconds();
    if duration.subsec_nanoseconds() < 0 {
        seconds - 1
    } else {
        seconds
    }
}

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
    use std::time::Duration as StdDuration;

    fn time(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + StdDuration::from_secs(secs)
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

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_ignores_filesystem_subsecond() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(100), Duration::new(100, 500_000_000)),
            Ordering::Equal,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_ignores_end_of_second() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(100), Duration::new(100, 999_999_999)),
            Ordering::Equal,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_is_older_at_next_second() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(100), Duration::seconds(101)),
            Ordering::Less,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_is_newer_at_previous_second() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(100), Duration::new(99, 999_999_999)),
            Ordering::Greater,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_ignores_filesystem_subsecond_before_epoch() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(-100), Duration::new(-99, -500_000_000)),
            Ordering::Equal,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_is_newer_below_its_own_second_before_epoch() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(-100), Duration::new(-100, -1)),
            Ordering::Greater,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_is_older_at_next_second_before_epoch() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(-100), Duration::seconds(-99)),
            Ordering::Less,
        );
    }

    #[test]
    fn cmp_at_stored_precision_whole_second_archive_accepts_exact_match_before_epoch() {
        assert_eq!(
            cmp_at_stored_precision(Duration::seconds(-100), Duration::seconds(-100)),
            Ordering::Equal,
        );
    }

    #[test]
    fn cmp_at_stored_precision_subsecond_archive_accepts_exact_match() {
        assert_eq!(
            cmp_at_stored_precision(
                Duration::new(100, 500_000_000),
                Duration::new(100, 500_000_000),
            ),
            Ordering::Equal,
        );
    }

    #[test]
    fn cmp_at_stored_precision_subsecond_archive_is_newer_within_same_second() {
        assert_eq!(
            cmp_at_stored_precision(
                Duration::new(100, 500_000_000),
                Duration::new(100, 250_000_000),
            ),
            Ordering::Greater,
        );
    }

    #[test]
    fn cmp_at_stored_precision_subsecond_archive_is_older_within_same_second() {
        assert_eq!(
            cmp_at_stored_precision(
                Duration::new(100, 500_000_000),
                Duration::new(100, 750_000_000),
            ),
            Ordering::Less,
        );
    }

    #[test]
    fn cmp_at_stored_precision_subsecond_archive_accepts_exact_match_before_epoch() {
        assert_eq!(
            cmp_at_stored_precision(
                Duration::new(-1, -500_000_000),
                Duration::new(-1, -500_000_000),
            ),
            Ordering::Equal,
        );
    }

    #[test]
    fn cmp_at_stored_precision_subsecond_archive_is_older_within_same_second_before_epoch() {
        assert_eq!(
            cmp_at_stored_precision(
                Duration::new(-1, -500_000_000),
                Duration::new(-1, -400_000_000),
            ),
            Ordering::Less,
        );
    }
}
