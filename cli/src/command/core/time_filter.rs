//! This module provides time-based filtering functionality for files.
//! It includes structs for defining time filters and applying them to file metadata.
use std::time::SystemTime;

use crate::cli::MissingTimePolicy;

/// Represents a filter based on time, allowing for inclusion of files newer or older than a specific `SystemTime`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TimeFilter {
    /// If `Some`, only includes files strictly newer than the specified `SystemTime`.
    pub(crate) newer_than: Option<SystemTime>,
    /// If `Some`, only includes files strictly older than the specified `SystemTime`.
    pub(crate) older_than: Option<SystemTime>,
    /// Controls behavior when the timestamp being filtered is absent from the entry.
    pub(crate) missing_policy: MissingTimePolicy,
}

impl TimeFilter {
    /// Checks if the time filter is active (i.e., if either `newer_than` or `older_than` is `Some`).
    ///
    /// # Returns
    ///
    /// `true` if the filter is active, `false` otherwise.
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        self.newer_than.is_some() || self.older_than.is_some()
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
        if let Some(newer) = self.newer_than
            && time <= newer
        {
            return false;
        }
        if let Some(older) = self.older_than
            && time >= older
        {
            return false;
        }
        true
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
    /// Checks if any of the time filters are active.
    ///
    /// # Returns
    ///
    /// `true` if either `ctime` or `mtime` filters are active, `false` otherwise.
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        self.ctime.is_active() || self.mtime.is_active()
    }

    /// Returns `true` if the given times pass both the ctime and mtime filters.
    pub(crate) fn matches(
        &self,
        fs_ctime: Option<SystemTime>,
        fs_mtime: Option<SystemTime>,
    ) -> bool {
        self.ctime.matches(fs_ctime) && self.mtime.matches(fs_mtime)
    }

    /// Returns `true` if no filters are active, or if the given times pass all active filters.
    #[inline]
    pub(crate) fn matches_or_inactive(
        &self,
        fs_ctime: Option<SystemTime>,
        fs_mtime: Option<SystemTime>,
    ) -> bool {
        !self.is_active() || self.matches(fs_ctime, fs_mtime)
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
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), Some(now())));
        assert!(filters.matches(None, None));
    }

    #[test]
    fn test_matches_newer_ctime() {
        // Case 1: newer_ctime is set, fs_ctime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(past()), Some(now())));

        // Case 2: newer_ctime is set, fs_ctime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: newer_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(None, Some(now())));
    }

    #[test]
    fn test_matches_older_ctime() {
        // Case 1: older_ctime is set, fs_ctime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(future()), Some(now())));

        // Case 2: older_ctime is set, fs_ctime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: older_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(None, Some(now())));
    }

    #[test]
    fn test_matches_newer_mtime() {
        // Case 1: newer_mtime is set, fs_mtime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(now()), Some(past())));

        // Case 2: newer_mtime is set, fs_mtime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: newer_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), None));
    }

    #[test]
    fn test_matches_older_mtime() {
        // Case 1: older_mtime is set, fs_mtime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(now()), Some(future())));

        // Case 2: older_mtime is set, fs_mtime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), Some(now())));

        // Case 3: older_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), None));
    }

    #[test]
    fn test_matches_all_filters_retain() {
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), Some(now())));
    }

    #[test]
    fn test_matches_all_filters_not_retain_ctime() {
        // newer_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(past()), Some(now())));

        // older_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(future()), Some(now())));
    }

    #[test]
    fn test_matches_all_filters_not_retain_mtime() {
        // newer_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(now()), Some(past())));

        // older_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!filters.matches(Some(now()), Some(future())));
    }

    #[test]
    fn test_matches_mixed_filters_and_none_fs_times() {
        // newer_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(None, Some(now())));

        // older_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(None, Some(now())));

        // newer_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), None));

        // older_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(filters.matches(Some(now()), None));
    }

    #[test]
    fn test_is_active() {
        let active = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(active.is_active());
        assert!(active.mtime.is_active());
        assert!(!active.ctime.is_active());

        let inactive = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
                missing_policy: MissingTimePolicy::Include,
            },
        };
        assert!(!inactive.is_active());
    }

    mod missing_policy {
        use super::*;
        use crate::cli::MissingTimePolicy;

        #[test]
        fn include_policy_passes_none_ctime() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: Some(now()),
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(filters.matches(None, Some(now())));
        }

        #[test]
        fn exclude_policy_rejects_none_ctime() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: Some(now()),
                    older_than: None,
                    missing_policy: MissingTimePolicy::Exclude,
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(!filters.matches(None, Some(now())));
        }

        #[test]
        fn exclude_policy_rejects_none_mtime() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
                mtime: TimeFilter {
                    newer_than: Some(now()),
                    older_than: None,
                    missing_policy: MissingTimePolicy::Exclude,
                },
            };
            assert!(!filters.matches(Some(now()), None));
        }

        #[test]
        fn assume_epoch_compared_as_older() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: Some(now()),
                    older_than: None,
                    missing_policy: MissingTimePolicy::Assume(past()),
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(!filters.matches(None, Some(now())));
        }

        #[test]
        fn assume_future_compared_as_newer() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: Some(now()),
                    older_than: None,
                    missing_policy: MissingTimePolicy::Assume(future()),
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(filters.matches(None, Some(now())));
        }

        #[test]
        fn assume_future_rejected_by_older_than() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: None,
                    older_than: Some(now()),
                    missing_policy: MissingTimePolicy::Assume(future()),
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(!filters.matches(None, Some(now())));
        }

        #[test]
        fn assume_past_passes_older_than() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: None,
                    older_than: Some(now()),
                    missing_policy: MissingTimePolicy::Assume(past()),
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(filters.matches(None, Some(now())));
        }

        #[test]
        fn policy_ignored_when_time_present() {
            let filters = TimeFilters {
                ctime: TimeFilter {
                    newer_than: Some(now()),
                    older_than: None,
                    missing_policy: MissingTimePolicy::Exclude,
                },
                mtime: TimeFilter {
                    newer_than: None,
                    older_than: None,
                    missing_policy: MissingTimePolicy::Include,
                },
            };
            assert!(filters.matches(Some(future()), Some(now())));
        }
    }
}
