//! This module provides time-based filtering functionality for files.
//! It includes structs for defining time filters and applying them to file metadata.
use std::{fs, time::SystemTime};

/// Represents a filter based on time, allowing for inclusion of files newer or older than a specific `SystemTime`.
#[derive(Clone, Debug)]
pub(crate) struct TimeFilter {
    /// If `Some`, only includes files strictly newer than the specified `SystemTime`.
    pub(crate) newer_than: Option<SystemTime>,
    /// If `Some`, only includes files strictly older than the specified `SystemTime`.
    pub(crate) older_than: Option<SystemTime>,
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

    /// Determines whether a file should be retained based on its modification time.
    ///
    /// # Arguments
    ///
    /// * `time` - An `Option<SystemTime>` representing the modification time of the file.
    ///
    /// # Returns
    ///
    /// `true` if the file should be retained, `false` otherwise.
    fn is_retain(&self, time: Option<SystemTime>) -> bool {
        if let Some(newer) = self.newer_than {
            if let Some(t) = time {
                if t <= newer {
                    return false;
                }
            }
        }
        if let Some(older) = self.older_than {
            if let Some(t) = time {
                if t >= older {
                    return false;
                }
            }
        }
        true
    }
}

/// Represents a set of time filters for creation time (`ctime`) and modification time (`mtime`).
#[derive(Clone, Debug)]
pub(crate) struct TimeFilters {
    /// A `TimeFilter` for the creation time of a file.
    pub(crate) ctime: TimeFilter,
    /// A `TimeFilter` for the modification time of a file.
    pub(crate) mtime: TimeFilter,
}

impl TimeFilters {
    /// Determines whether a file should be retained based on its metadata.
    ///
    /// # Arguments
    ///
    /// * `fs_meta` - A reference to the file's `fs::Metadata`.
    ///
    /// # Returns
    ///
    /// `true` if the file should be retained, `false` otherwise.
    #[inline]
    pub(crate) fn is_retain(&self, fs_meta: &fs::Metadata) -> bool {
        self.is_retain_t(fs_meta.created().ok(), fs_meta.modified().ok())
    }

    /// Checks if any of the time filters are active.
    ///
    /// # Returns
    ///
    /// `true` if either `ctime` or `mtime` filters are active, `false` otherwise.
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        self.ctime.is_active() || self.mtime.is_active()
    }

    /// Determines whether a file should be retained based on its creation and modification times.
    ///
    /// # Arguments
    ///
    /// * `fs_ctime` - An `Option<SystemTime>` for the file's creation time.
    /// * `fs_mtime` - An `Option<SystemTime>` for the file's modification time.
    ///
    /// # Returns
    ///
    /// `true` if the file should be retained, `false` otherwise.
    fn is_retain_t(&self, fs_ctime: Option<SystemTime>, fs_mtime: Option<SystemTime>) -> bool {
        self.ctime.is_retain(fs_ctime) && self.mtime.is_retain(fs_mtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_is_retain_t_no_filters() {
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));
        assert!(filters.is_retain_t(None, None));
    }

    #[test]
    fn test_is_retain_t_newer_ctime() {
        // Case 1: newer_ctime is set, fs_ctime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(!filters.is_retain_t(Some(past()), Some(now())));

        // Case 2: newer_ctime is set, fs_ctime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: newer_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));
    }

    #[test]
    fn test_is_retain_t_older_ctime() {
        // Case 1: older_ctime is set, fs_ctime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(!filters.is_retain_t(Some(future()), Some(now())));

        // Case 2: older_ctime is set, fs_ctime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: older_ctime is set, fs_ctime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));
    }

    #[test]
    fn test_is_retain_t_newer_mtime() {
        // Case 1: newer_mtime is set, fs_mtime is older -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(past())));

        // Case 2: newer_mtime is set, fs_mtime is newer -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: newer_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));
    }

    #[test]
    fn test_is_retain_t_older_mtime() {
        // Case 1: older_mtime is set, fs_mtime is newer -> should not retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(future())));

        // Case 2: older_mtime is set, fs_mtime is older -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(future()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));

        // Case 3: older_mtime is set, fs_mtime is None -> should retain
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));
    }

    #[test]
    fn test_is_retain_t_all_filters_retain() {
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), Some(now())));
    }

    #[test]
    fn test_is_retain_t_all_filters_not_retain_ctime() {
        // newer_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
        };
        assert!(!filters.is_retain_t(Some(past()), Some(now())));

        // older_ctime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
        };
        assert!(!filters.is_retain_t(Some(future()), Some(now())));
    }

    #[test]
    fn test_is_retain_t_all_filters_not_retain_mtime() {
        // newer_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: Some(future()),
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(past())));

        // older_mtime fails
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(future()),
            },
            mtime: TimeFilter {
                newer_than: Some(past()),
                older_than: Some(now()),
            },
        };
        assert!(!filters.is_retain_t(Some(now()), Some(future())));
    }

    #[test]
    fn test_is_retain_t_mixed_filters_and_none_fs_times() {
        // newer_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));

        // older_ctime set, fs_ctime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(None, Some(now())));

        // newer_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));

        // older_mtime set, fs_mtime is None
        let filters = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: Some(now()),
            },
        };
        assert!(filters.is_retain_t(Some(now()), None));
    }

    #[test]
    fn test_is_active() {
        let active = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: Some(now()),
                older_than: None,
            },
        };
        assert!(active.is_active());
        assert!(active.mtime.is_active());
        assert!(!active.ctime.is_active());

        let inactive = TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        };
        assert!(!inactive.is_active());
    }
}
