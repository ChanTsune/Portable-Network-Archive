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
const MTIME_2001: Duration = Duration::seconds(978_307_200);
const ARCHIVED_MTIME_SECS: u64 = 1_579_046_400;
const ARCHIVED_ATIME_SECS: u64 = 1_579_132_800;

#[derive(Clone, Copy)]
enum ExpectedModified {
    Preserved,
    Override(Duration),
}

struct Case {
    name: &'static str,
    flags: &'static [&'static str],
    modified: ExpectedModified,
}

const CASES: &[Case] = &[
    Case {
        name: "mtime",
        flags: &["--mtime", "2001-01-01T00:00:00Z"],
        modified: ExpectedModified::Override(MTIME_2001),
    },
    Case {
        name: "clamp-newer",
        flags: &["--mtime", "2001-01-01T00:00:00Z", "--clamp-mtime"],
        modified: ExpectedModified::Override(MTIME_2001),
    },
    Case {
        name: "clamp-older",
        flags: &["--mtime", "2025-01-01T00:00:00Z", "--clamp-mtime"],
        modified: ExpectedModified::Preserved,
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

fn resolved_modified(expected: ExpectedModified, preserved: Duration) -> Option<Duration> {
    match expected {
        ExpectedModified::Preserved => Some(preserved),
        ExpectedModified::Override(value) => Some(value),
    }
}

/// Precondition: A file with known timestamps is archived by bsdtar create.
/// Action: Create with `--mtime` and optional `--clamp-mtime`.
/// Expectation: mtime is overridden or clamped while the other source timestamps are implicitly preserved.
#[test]
fn bsdtar_create_mtime() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_create_mtime_{index}");
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
            (
                created,
                resolved_modified(case.modified, Duration::seconds(SOURCE_MTIME_SECS as i64)),
                Some(Duration::seconds(SOURCE_ATIME_SECS as i64)),
            ),
        );
    }
}

/// Precondition: An archive without timestamp metadata exists and a file with known timestamps is appended.
/// Action: Append with `--mtime` and optional `--clamp-mtime`.
/// Expectation: mtime is overridden or clamped while the other source timestamps are implicitly preserved.
#[test]
fn bsdtar_append_mtime() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_append_mtime_{index}");
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
            (
                created,
                resolved_modified(case.modified, Duration::seconds(SOURCE_MTIME_SECS as i64)),
                Some(Duration::seconds(SOURCE_ATIME_SECS as i64)),
            ),
        );
    }
}

/// Precondition: An archive stores a file with source timestamps and the file is modified.
/// Action: Update with `--mtime` and optional `--clamp-mtime`.
/// Expectation: The revised entry overrides or clamps mtime while the other source timestamps are implicitly preserved.
#[test]
fn bsdtar_update_mtime() {
    setup();
    for (index, case) in CASES.iter().enumerate() {
        let base = format!("bsdtar_update_mtime_{index}");
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
            (
                original_created,
                Some(Duration::seconds(SOURCE_MTIME_SECS as i64)),
                Some(Duration::seconds(SOURCE_ATIME_SECS as i64))
            ),
            "{} original entry",
            case.name
        );
        assert_eq!(
            times[1],
            (
                revised_created,
                resolved_modified(case.modified, Duration::seconds(UPDATED_MTIME_SECS as i64)),
                Some(Duration::seconds(UPDATED_ATIME_SECS as i64))
            ),
            "{} revised entry",
            case.name
        );
    }
}

struct ExtractCase {
    name: &'static str,
    flags: &'static [&'static str],
    expected_mtime: u64,
}

const EXTRACT_CASES: &[ExtractCase] = &[
    ExtractCase {
        name: "mtime",
        flags: &["--mtime", "2024-06-01T00:00:00Z"],
        expected_mtime: 1_717_200_000,
    },
    ExtractCase {
        name: "clamp-newer",
        flags: &["--mtime", "2019-01-01T00:00:00Z", "--clamp-mtime"],
        expected_mtime: 1_546_300_800,
    },
    ExtractCase {
        name: "clamp-older",
        flags: &["--mtime", "2025-01-01T00:00:00Z", "--clamp-mtime"],
        expected_mtime: ARCHIVED_MTIME_SECS,
    },
];

/// Precondition: An archive stores a file with explicit mtime and atime.
/// Action: Extract with `--mtime` and optional `--clamp-mtime`.
/// Expectation: mtime is overridden or clamped while the archived atime is implicitly preserved.
#[test]
fn bsdtar_extract_mtime() {
    setup();
    for (index, case) in EXTRACT_CASES.iter().enumerate() {
        let base = format!("bsdtar_extract_mtime_{index}");
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

        let metadata = fs::metadata(format!("{out_dir}/{source}")).unwrap();
        assert_same_second(
            metadata.modified().unwrap(),
            case.expected_mtime,
            "mtime",
            case.name,
        );
        assert_same_second(
            metadata.accessed().unwrap(),
            ARCHIVED_ATIME_SECS,
            "atime",
            case.name,
        );
    }
}

fn assert_same_second(actual: SystemTime, expected_secs: u64, label: &str, case: &str) {
    let actual = actual
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(actual, expected_secs, "{label} mismatch in {case}");
}
