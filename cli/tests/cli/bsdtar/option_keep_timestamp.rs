#![cfg(not(target_family = "wasm"))]
use crate::utils::{
    archive, setup,
    time::{set_file_times, stored_time},
};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::Duration;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

const SOURCE_MTIME_SECS: u64 = 1_275_782_400;
const SOURCE_ATIME_SECS: u64 = 1_275_868_800;
const UPDATED_MTIME_SECS: u64 = 1_600_000_000;
const UPDATED_ATIME_SECS: u64 = 1_600_086_400;
const ARCHIVED_MTIME_SECS: u64 = 1_579_046_400;
const ARCHIVED_ATIME_SECS: u64 = 1_579_132_800;
const SOURCE_MTIME: Duration = Duration::seconds(SOURCE_MTIME_SECS as i64);
const SOURCE_ATIME: Duration = Duration::seconds(SOURCE_ATIME_SECS as i64);
const UPDATED_MTIME: Duration = Duration::seconds(UPDATED_MTIME_SECS as i64);
const UPDATED_ATIME: Duration = Duration::seconds(UPDATED_ATIME_SECS as i64);

#[derive(Clone, Copy)]
enum Expected {
    Preserved,
    Absent,
}

struct FlagCase {
    name: &'static str,
    flags: &'static [&'static str],
    expected: Expected,
}

const CASES: &[FlagCase] = &[
    FlagCase {
        name: "default",
        flags: &[],
        expected: Expected::Preserved,
    },
    FlagCase {
        name: "preserve",
        flags: &["--preserve-timestamps"],
        expected: Expected::Preserved,
    },
    FlagCase {
        name: "keep-alias",
        flags: &["--keep-timestamp"],
        expected: Expected::Preserved,
    },
    FlagCase {
        name: "negative",
        flags: &["--no-preserve-timestamps"],
        expected: Expected::Absent,
    },
    FlagCase {
        name: "negative-mtime",
        flags: &[
            "--no-preserve-timestamps",
            "--mtime",
            "2001-01-01T00:00:00Z",
        ],
        expected: Expected::Absent,
    },
    FlagCase {
        name: "negative-ctime",
        flags: &[
            "--no-preserve-timestamps",
            "--ctime",
            "2001-01-01T00:00:00Z",
        ],
        expected: Expected::Absent,
    },
    FlagCase {
        name: "negative-atime",
        flags: &[
            "--no-preserve-timestamps",
            "--atime",
            "2001-01-01T00:00:00Z",
        ],
        expected: Expected::Absent,
    },
    FlagCase {
        name: "negative-alias",
        flags: &["--no-keep-timestamp"],
        expected: Expected::Absent,
    },
    FlagCase {
        name: "modification-time",
        flags: &["--modification-time"],
        expected: Expected::Absent,
    },
    FlagCase {
        name: "short-m",
        flags: &["-m"],
        expected: Expected::Absent,
    },
];

fn set_source_times(path: &str, mtime: u64, atime: u64) {
    set_file_times(
        path,
        UNIX_EPOCH + std::time::Duration::from_secs(mtime),
        UNIX_EPOCH + std::time::Duration::from_secs(atime),
    );
}

fn source_created(path: &str) -> Option<Duration> {
    fs::metadata(path).unwrap().created().ok().map(stored_time)
}

fn assert_entry_times(
    archive_path: &str,
    name: &str,
    expected: (Option<Duration>, Option<Duration>, Option<Duration>),
) {
    let mut found = false;
    archive::for_each_entry(archive_path, |entry| {
        if entry.header().path().as_str().ends_with(name) {
            found = true;
            assert_eq!(
                (
                    entry.metadata().created(),
                    entry.metadata().modified(),
                    entry.metadata().accessed()
                ),
                expected,
                "{name}"
            );
        }
    })
    .unwrap();
    assert!(found, "{name} entry");
}

fn entry_times(
    archive_path: &str,
    name: &str,
) -> Vec<(Option<Duration>, Option<Duration>, Option<Duration>)> {
    let mut times = Vec::new();
    archive::for_each_entry(archive_path, |entry| {
        if entry.header().path().as_str().ends_with(name) {
            times.push((
                entry.metadata().created(),
                entry.metadata().modified(),
                entry.metadata().accessed(),
            ));
        }
    })
    .unwrap();
    times
}

fn expected_times(
    expected: Expected,
    created: Option<Duration>,
) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
    match expected {
        Expected::Preserved => (created, Some(SOURCE_MTIME), Some(SOURCE_ATIME)),
        Expected::Absent => (None, None, None),
    }
}

fn expected_updated_times(
    expected: Expected,
    created: Option<Duration>,
) -> (Option<Duration>, Option<Duration>, Option<Duration>) {
    match expected {
        Expected::Preserved => (created, Some(UPDATED_MTIME), Some(UPDATED_ATIME)),
        Expected::Absent => (None, None, None),
    }
}

/// Precondition: A file with known timestamps is archived by bsdtar create.
/// Action: Create with each timestamp preservation flag spelling.
/// Expectation: bsdtar preserves timestamps by default and for positive flags, while negative flags omit them.
#[test]
fn bsdtar_create_timestamp_flags() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_create_timestamp_flags_{index}");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let source = format!("{base}/source.txt");
        fs::write(&source, b"content").unwrap();
        set_source_times(&source, SOURCE_MTIME_SECS, SOURCE_ATIME_SECS);
        let created = source_created(&source);
        let archive_path = format!("{base}/archive.pna");

        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args(["compat", "bsdtar", "-c", "-f", &archive_path, &source]);
        cmd.args(case.flags);
        cmd.assert().success();

        assert_entry_times(
            &archive_path,
            "source.txt",
            expected_times(case.expected, created),
        );
    }
}

/// Precondition: An archive without timestamp metadata exists and a file with known timestamps is appended.
/// Action: Append with each timestamp preservation flag spelling.
/// Expectation: bsdtar preserves appended timestamps by default and for positive flags, while negative flags omit them.
#[test]
fn bsdtar_append_timestamp_flags() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_append_timestamp_flags_{index}");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let seed = format!("{base}/seed.txt");
        fs::write(&seed, b"seed").unwrap();
        let archive_path = format!("{base}/archive.pna");
        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args(["c", "-f", &archive_path, "--overwrite", &seed])
            .assert()
            .success();

        let added = format!("{base}/added.txt");
        fs::write(&added, b"added").unwrap();
        set_source_times(&added, SOURCE_MTIME_SECS, SOURCE_ATIME_SECS);
        let created = source_created(&added);

        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args(["compat", "bsdtar", "-r", "-f", &archive_path, &added]);
        cmd.args(case.flags);
        cmd.assert().success();

        assert_entry_times(&archive_path, "seed.txt", (None, None, None));
        assert_entry_times(
            &archive_path,
            "added.txt",
            expected_times(case.expected, created),
        );
    }
}

/// Precondition: An archive stores a file with source timestamps and the file is modified.
/// Action: Update with each timestamp preservation flag spelling.
/// Expectation: The original entry keeps its timestamps; the revised entry preserves updated timestamps or omits them.
#[test]
fn bsdtar_update_timestamp_flags() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_update_timestamp_flags_{index}");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let source = format!("{base}/source.txt");
        fs::write(&source, b"v1").unwrap();
        set_source_times(&source, SOURCE_MTIME_SECS, SOURCE_ATIME_SECS);
        let original_created = source_created(&source);
        let archive_path = format!("{base}/archive.pna");
        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args([
            "c",
            "-f",
            &archive_path,
            "--overwrite",
            "--preserve-timestamps",
            &source,
        ])
        .assert()
        .success();

        fs::write(&source, b"v2 with different content").unwrap();
        set_source_times(&source, UPDATED_MTIME_SECS, UPDATED_ATIME_SECS);
        let revised_created = source_created(&source);

        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args(["compat", "bsdtar", "-u", "-f", &archive_path, &source]);
        cmd.args(case.flags);
        cmd.assert().success();

        let times = entry_times(&archive_path, "source.txt");
        assert_eq!(times.len(), 2, "{}", case.name);
        assert_eq!(
            times[0],
            (original_created, Some(SOURCE_MTIME), Some(SOURCE_ATIME)),
            "{} original entry",
            case.name
        );
        assert_eq!(
            times[1],
            expected_updated_times(case.expected, revised_created),
            "{} revised entry",
            case.name
        );
    }
}

/// Precondition: An archive stores a file with explicit mtime and atime.
/// Action: Extract with each timestamp preservation flag spelling.
/// Expectation: bsdtar restores timestamps by default and for positive flags, while negative flags use the extraction time.
#[test]
fn bsdtar_extract_timestamp_flags() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_extract_timestamp_flags_{index}");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        let source = format!("{base}/source.txt");
        fs::write(&source, b"content").unwrap();
        set_source_times(&source, SOURCE_MTIME_SECS, SOURCE_ATIME_SECS);
        let archive_path = format!("{base}/archive.pna");
        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args([
            "c",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            "--mtime",
            "2020-01-15T00:00:00Z",
            "--atime",
            "2020-01-16T00:00:00Z",
            &source,
        ])
        .assert()
        .success();

        let out_dir = format!("{base}/out");
        fs::create_dir_all(&out_dir).unwrap();
        let mut cmd = cargo_bin_cmd!("pna");
        cmd.args([
            "compat",
            "bsdtar",
            "-x",
            "-f",
            &archive_path,
            "-C",
            &out_dir,
        ]);
        cmd.args(case.flags);
        cmd.assert().success();

        let extracted = format!("{out_dir}/{source}");
        let metadata = fs::metadata(&extracted).unwrap();
        let modified = metadata.modified().unwrap();
        let accessed = metadata.accessed().unwrap();
        match case.expected {
            Expected::Preserved => {
                assert_same_second(modified, ARCHIVED_MTIME_SECS, "mtime", case.name);
                assert_same_second(accessed, ARCHIVED_ATIME_SECS, "atime", case.name);
            }
            Expected::Absent => {
                assert_after_second(modified, ARCHIVED_MTIME_SECS, "mtime", case.name);
                assert_after_second(accessed, ARCHIVED_ATIME_SECS, "atime", case.name);
            }
        }
    }
}

fn assert_same_second(actual: SystemTime, expected_secs: u64, label: &str, case: &str) {
    let actual = actual
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(actual, expected_secs, "{label} mismatch in {case}");
}

fn assert_after_second(actual: SystemTime, expected_secs: u64, label: &str, case: &str) {
    let actual = actual
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(
        actual > expected_secs,
        "{label} should be extraction time in {case}, got {actual} (archived {expected_secs})"
    );
}
