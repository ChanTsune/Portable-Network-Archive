use std::{
    fs, thread,
    time::{Duration, SystemTime},
};

pub const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

#[cfg(not(target_family = "wasm"))]
#[track_caller]
pub fn set_mtime(path: &str, at: SystemTime) {
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(at)).unwrap();
}

#[track_caller]
pub fn birth_time(path: &str) -> SystemTime {
    fs::metadata(path).unwrap().created().unwrap()
}

/// `created()` also succeeds on file systems that keep no birth time and answer
/// with the epoch for every file (OpenBSD FFS), where no amount of re-creating
/// can order two files. Such a value means the capability is absent, not that a
/// file predates the epoch.
#[track_caller]
pub fn birth_time_recorded(path: &str) -> bool {
    fs::metadata(path)
        .unwrap()
        .created()
        .is_ok_and(|born| born > SystemTime::UNIX_EPOCH)
}

/// Birth time cannot be set, only observed, and it is fixed at creation, so
/// ordering against `baseline` is obtained by re-creating the file until its
/// birth time is strictly later.
///
/// Each attempt is written to a fresh path rather than `path` itself: on NTFS,
/// re-creating the same name within roughly 15 seconds of deleting it restores
/// the original creation time (file-system tunneling), so retrying in place
/// would observe the same birth time forever. The winning attempt is renamed
/// onto `path`, which preserves its creation time on both NTFS and Unix; the
/// birth time is read back afterwards because tunneling applies to the
/// destination name too when `path` already existed.
#[track_caller]
pub fn create_file_born_after(path: &str, content: &str, baseline: SystemTime) -> SystemTime {
    const MAX_ATTEMPTS: usize = 300;
    for attempt in 0..MAX_ATTEMPTS {
        let attempt_path = format!("{path}.attempt{attempt}");
        fs::write(&attempt_path, content).unwrap();
        let born = birth_time(&attempt_path);
        if born > baseline {
            fs::rename(&attempt_path, path).unwrap();
            let renamed = birth_time(path);
            assert!(
                renamed > baseline,
                "{path}: renaming onto an existing name reverted the birth time to {renamed:?}"
            );
            return renamed;
        }
        fs::remove_file(&attempt_path).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "{path}: could not obtain a birth time later than {baseline:?} in {MAX_ATTEMPTS} attempts"
    );
}
