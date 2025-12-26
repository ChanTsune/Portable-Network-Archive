use std::{
    fs, thread,
    time::{Duration, SystemTime},
};

/// Waits until the file at `path` has a timestamp newer than `baseline`.
/// The `get_time` function extracts the relevant timestamp from metadata.
pub fn wait_until_time_newer_than<F>(path: &str, baseline: SystemTime, get_time: F) -> bool
where
    F: Fn(&fs::Metadata) -> Option<SystemTime>,
{
    const MAX_ATTEMPTS: usize = 500;
    const SLEEP_MS: u64 = 10;
    for _ in 0..MAX_ATTEMPTS {
        if fs::metadata(path)
            .ok()
            .and_then(|meta| get_time(&meta))
            .map(|time| time > baseline)
            .unwrap_or(false)
        {
            return true;
        }
        thread::sleep(Duration::from_millis(SLEEP_MS));
    }
    false
}

/// Confirms that the file at `path` has a timestamp older than `baseline`.
/// The `get_time` function extracts the relevant timestamp from metadata.
pub fn confirm_time_older_than<F>(path: &str, baseline: SystemTime, get_time: F) -> bool
where
    F: Fn(&fs::Metadata) -> Option<SystemTime>,
{
    fs::metadata(path)
        .ok()
        .and_then(|meta| get_time(&meta))
        .map(|time| time < baseline)
        .unwrap_or(false)
}

/// Ensures that `older` file has a ctime older than `reference` and
/// `newer` file has a ctime newer than `reference`.
pub fn ensure_ctime_order(older: &str, newer: &str, reference: SystemTime) -> bool {
    if !confirm_time_older_than(older, reference, |m| m.created().ok()) {
        return false;
    }
    wait_until_time_newer_than(newer, reference, |m| m.created().ok())
}

/// Ensures that `older` file has an mtime older than `reference` and
/// `newer` file has an mtime newer than `reference`.
pub fn ensure_mtime_order(older: &str, newer: &str, reference: SystemTime) -> bool {
    if !confirm_time_older_than(older, reference, |m| m.modified().ok()) {
        return false;
    }
    wait_until_time_newer_than(newer, reference, |m| m.modified().ok())
}
