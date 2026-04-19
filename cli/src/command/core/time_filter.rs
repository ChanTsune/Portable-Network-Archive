//! This module provides time-based filtering functionality for files.
//! It includes structs for defining time filters and applying them to file metadata.
use std::time::SystemTime;

use crate::cli::MissingTimePolicy;

mod range {
    use std::ops::Bound;
    use std::time::SystemTime;

    /// Time range with boundary inclusivity expressed via `Bound`.
    ///
    /// # Invariants
    ///
    /// - The relation between `start` and `end` is not enforced. An inverted
    ///   range (e.g. `Excluded(t_later)..Excluded(t_earlier)`) is permitted
    ///   at construction time and `contains` returns `false` for every
    ///   input (empty-range semantics).
    /// - This preserves the legacy `TimeFilter::matches` behavior where an
    ///   inverted `(newer, older)` pair passed to `new_strict` silently
    ///   matches nothing.
    ///
    /// # Boundary semantics
    ///
    /// - `Bound::Excluded(v)` rejects `time == v`.
    /// - `Bound::Included(v)` accepts `time == v`.
    /// - `Bound::Unbounded` imposes no constraint on that side.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct TimeRange {
        start: Bound<SystemTime>,
        end: Bound<SystemTime>,
    }

    impl TimeRange {
        /// Build a range with strict boundaries from the `(newer, older)`
        /// pair used by `TimeFilter`.
        ///
        /// - `newer = Some(t)` becomes `start = Bound::Excluded(t)`
        /// - `newer = None`    becomes `start = Bound::Unbounded`
        /// - `older = Some(t)` becomes `end = Bound::Excluded(t)`
        /// - `older = None`    becomes `end = Bound::Unbounded`
        ///
        /// Matches the strict `>`/`<` semantics fixed in commit `befbdb86`.
        pub(crate) fn new_strict(newer: Option<SystemTime>, older: Option<SystemTime>) -> Self {
            Self {
                start: newer.map_or(Bound::Unbounded, Bound::Excluded),
                end: older.map_or(Bound::Unbounded, Bound::Excluded),
            }
        }

        pub(crate) fn contains(&self, time: SystemTime) -> bool {
            let start_ok = match self.start {
                Bound::Unbounded => true,
                Bound::Included(s) => time >= s,
                Bound::Excluded(s) => time > s,
            };
            let end_ok = match self.end {
                Bound::Unbounded => true,
                Bound::Included(e) => time <= e,
                Bound::Excluded(e) => time < e,
            };
            start_ok && end_ok
        }

        #[cfg(test)]
        pub(crate) fn from_bounds(start: Bound<SystemTime>, end: Bound<SystemTime>) -> Self {
            Self { start, end }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{Duration, UNIX_EPOCH};

        fn t1() -> SystemTime {
            UNIX_EPOCH
        }
        fn t2() -> SystemTime {
            UNIX_EPOCH + Duration::from_secs(3600)
        }
        fn t3() -> SystemTime {
            UNIX_EPOCH + Duration::from_secs(7200)
        }

        #[test]
        fn contains_unbounded_passes_any_time() {
            let r = TimeRange::from_bounds(Bound::Unbounded, Bound::Unbounded);
            assert!(r.contains(t1()));
            assert!(r.contains(t2()));
            assert!(r.contains(t3()));
        }

        #[test]
        fn contains_excluded_start_rejects_equal() {
            let r = TimeRange::from_bounds(Bound::Excluded(t2()), Bound::Unbounded);
            assert!(!r.contains(t2()));
        }

        #[test]
        fn contains_excluded_start_accepts_later() {
            let r = TimeRange::from_bounds(Bound::Excluded(t2()), Bound::Unbounded);
            assert!(r.contains(t3()));
            assert!(!r.contains(t1()));
        }

        #[test]
        fn contains_excluded_end_rejects_equal() {
            let r = TimeRange::from_bounds(Bound::Unbounded, Bound::Excluded(t2()));
            assert!(!r.contains(t2()));
        }

        #[test]
        fn contains_excluded_end_accepts_earlier() {
            let r = TimeRange::from_bounds(Bound::Unbounded, Bound::Excluded(t2()));
            assert!(r.contains(t1()));
            assert!(!r.contains(t3()));
        }

        #[test]
        fn contains_included_start_accepts_equal() {
            let r = TimeRange::from_bounds(Bound::Included(t2()), Bound::Unbounded);
            assert!(r.contains(t2()));
            assert!(r.contains(t3()));
            assert!(!r.contains(t1()));
        }

        #[test]
        fn contains_included_end_accepts_equal() {
            let r = TimeRange::from_bounds(Bound::Unbounded, Bound::Included(t2()));
            assert!(r.contains(t2()));
            assert!(r.contains(t1()));
            assert!(!r.contains(t3()));
        }

        #[test]
        fn new_strict_both_none_passes_any_time() {
            let r = TimeRange::new_strict(None, None);
            assert!(r.contains(t1()));
            assert!(r.contains(t2()));
            assert!(r.contains(t3()));
        }

        #[test]
        fn new_strict_newer_maps_to_excluded_start() {
            // newer = Some(t2) => start = Excluded(t2)
            let r = TimeRange::new_strict(Some(t2()), None);
            assert!(!r.contains(t1()));
            assert!(!r.contains(t2())); // equal rejected (strict)
            assert!(r.contains(t3()));
        }

        #[test]
        fn new_strict_older_maps_to_excluded_end() {
            // older = Some(t2) => end = Excluded(t2)
            let r = TimeRange::new_strict(None, Some(t2()));
            assert!(r.contains(t1()));
            assert!(!r.contains(t2())); // equal rejected (strict)
            assert!(!r.contains(t3()));
        }

        #[test]
        fn contains_inverted_range_rejects_all() {
            // start > end: empty-range semantics
            let r = TimeRange::from_bounds(Bound::Excluded(t3()), Bound::Excluded(t1()));
            assert!(!r.contains(t1()));
            assert!(!r.contains(t2()));
            assert!(!r.contains(t3()));
        }

        #[test]
        fn contains_both_bounded_accepts_strictly_inside() {
            let r = TimeRange::from_bounds(Bound::Excluded(t1()), Bound::Excluded(t3()));
            assert!(r.contains(t2()));
            assert!(!r.contains(t1()));
            assert!(!r.contains(t3()));
        }

        #[test]
        fn contains_both_included_accepts_both_bounds_and_between() {
            // Closes the Included..Included coverage gap noted in Task 3 review.
            let r = TimeRange::from_bounds(Bound::Included(t1()), Bound::Included(t3()));
            assert!(r.contains(t1())); // lower bound accepted (inclusive)
            assert!(r.contains(t2())); // strictly inside
            assert!(r.contains(t3())); // upper bound accepted (inclusive)
        }
    }
}

pub(crate) use range::TimeRange;

/// Represents a filter based on time, composing a `TimeRange` with a fallback policy
/// for entries missing the compared timestamp.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TimeFilter {
    range: TimeRange,
    missing_policy: MissingTimePolicy,
}

impl TimeFilter {
    /// Build a filter from a range and a missing-timestamp policy.
    #[inline]
    pub(crate) fn new(range: TimeRange, missing_policy: MissingTimePolicy) -> Self {
        Self {
            range,
            missing_policy,
        }
    }

    /// Determines whether a given timestamp passes this filter's constraints.
    fn matches(&self, time: Option<SystemTime>) -> bool {
        let time = match time {
            Some(t) => t,
            None => match &self.missing_policy {
                MissingTimePolicy::Include => return true,
                MissingTimePolicy::Exclude => return false,
                MissingTimePolicy::Assume(t) => *t,
            },
        };
        self.range.contains(time)
    }
}

/// Represents a set of time filters for creation time (`ctime`) and modification time (`mtime`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct TimeFilters {
    /// A `TimeFilter` for the creation time of a file.
    pub(crate) ctime: TimeFilter,
    /// A `TimeFilter` for the modification time of a file.
    pub(crate) mtime: TimeFilter,
}

impl TimeFilters {
    /// Returns `true` if the given times pass both the ctime and mtime filters.
    pub(crate) fn matches(
        &self,
        fs_ctime: Option<SystemTime>,
        fs_mtime: Option<SystemTime>,
    ) -> bool {
        self.ctime.matches(fs_ctime) && self.mtime.matches(fs_mtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::MissingTimePolicy;
    use std::time::Duration;

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(3600)
    }

    fn past() -> SystemTime {
        SystemTime::UNIX_EPOCH
    }

    fn future() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(7200)
    }

    #[test]
    fn test_matches_no_filters() {
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), Some(now())));
        assert!(filters.matches(None, None));
    }

    #[test]
    fn test_matches_newer_ctime() {
        // Case 1: newer_ctime is set, fs_ctime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(past()), Some(now())));

        // Case 2: newer_ctime is set, fs_ctime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: newer_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(None, Some(now())));
    }

    #[test]
    fn test_matches_older_ctime() {
        // Case 1: older_ctime is set, fs_ctime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, Some(now())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(future()), Some(now())));

        // Case 2: older_ctime is set, fs_ctime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, Some(future())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: older_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, Some(now())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(None, Some(now())));
    }

    #[test]
    fn test_matches_newer_mtime() {
        // Case 1: newer_mtime is set, fs_mtime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(now()), Some(past())));

        // Case 2: newer_mtime is set, fs_mtime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: newer_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), None));
    }

    #[test]
    fn test_matches_older_mtime() {
        // Case 1: older_mtime is set, fs_mtime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, Some(now())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(now()), Some(future())));

        // Case 2: older_mtime is set, fs_mtime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, Some(future())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: older_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, Some(now())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), None));
    }

    #[test]
    fn test_matches_all_filters_retain() {
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(future())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(future())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), Some(now())));
    }

    #[test]
    fn test_matches_all_filters_not_retain_ctime() {
        // newer_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), Some(future())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(future())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(past()), Some(now())));

        // older_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(now())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(future())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(future()), Some(now())));
    }

    #[test]
    fn test_matches_all_filters_not_retain_mtime() {
        // newer_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(future())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), Some(future())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(now()), Some(past())));

        // older_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(future())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(past()), Some(now())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(!filters.matches(Some(now()), Some(future())));
    }

    #[test]
    fn test_matches_mixed_filters_and_none_fs_times() {
        // newer_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(None, Some(now())));

        // older_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, Some(now())),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(None, Some(now())));

        // newer_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(Some(now()), None),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), None));

        // older_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter::new(
                TimeRange::new_strict(None, None),
                MissingTimePolicy::Include,
            ),
            mtime: TimeFilter::new(
                TimeRange::new_strict(None, Some(now())),
                MissingTimePolicy::Include,
            ),
        };
        assert!(filters.matches(Some(now()), None));
    }

    mod missing_policy {
        use super::*;
        use crate::cli::MissingTimePolicy;

        #[test]
        fn include_policy_passes_none_ctime() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(Some(now()), None),
                    MissingTimePolicy::Include,
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(filters.matches(None, Some(now())));
        }

        #[test]
        fn exclude_policy_rejects_none_ctime() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(Some(now()), None),
                    MissingTimePolicy::Exclude,
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(!filters.matches(None, Some(now())));
        }

        #[test]
        fn exclude_policy_rejects_none_mtime() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(Some(now()), None),
                    MissingTimePolicy::Exclude,
                ),
            };
            assert!(!filters.matches(Some(now()), None));
        }

        #[test]
        fn assume_epoch_compared_as_older() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(Some(now()), None),
                    MissingTimePolicy::Assume(past()),
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(!filters.matches(None, Some(now())));
        }

        #[test]
        fn assume_future_compared_as_newer() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(Some(now()), None),
                    MissingTimePolicy::Assume(future()),
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(filters.matches(None, Some(now())));
        }

        #[test]
        fn assume_future_rejected_by_older_than() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(None, Some(now())),
                    MissingTimePolicy::Assume(future()),
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(!filters.matches(None, Some(now())));
        }

        #[test]
        fn assume_past_passes_older_than() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(None, Some(now())),
                    MissingTimePolicy::Assume(past()),
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(filters.matches(None, Some(now())));
        }

        #[test]
        fn policy_ignored_when_time_present() {
            let filters = TimeFilters {
                ctime: TimeFilter::new(
                    TimeRange::new_strict(Some(now()), None),
                    MissingTimePolicy::Exclude,
                ),
                mtime: TimeFilter::new(
                    TimeRange::new_strict(None, None),
                    MissingTimePolicy::Include,
                ),
            };
            assert!(filters.matches(Some(future()), Some(now())));
        }
    }
}
