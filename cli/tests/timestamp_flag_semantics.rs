//! Pin the observable timestamp flag semantics of the CLI: which timestamps
//! archives store and extraction restores, for each combination of the
//! preserve/no-preserve flags and the time override options.
//!
//! Extraction restores the modification and access times; the stored
//! creation time has no filesystem counterpart to restore into here.

#![cfg(not(target_family = "wasm"))]

use pna::{Archive, Duration as StoredDuration, ReadEntry};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const SOURCE_TIME: Duration = Duration::from_secs(1_275_782_400);
const UPDATED_TIME: Duration = Duration::from_secs(1_600_000_000);
const MTIME_2001: Duration = Duration::from_secs(978_307_200);
const CTIME_2002: Duration = Duration::from_secs(1_009_843_200);
const ATIME_2003: Duration = Duration::from_secs(1_041_379_200);
const YEAR_2000: Duration = Duration::from_secs(946_684_800);
const MTIME_2001_ARG: &str = "2001-01-01T00:00:00Z";
const CTIME_2002_ARG: &str = "2002-01-01T00:00:00Z";
const ATIME_2003_ARG: &str = "2003-01-01T00:00:00Z";
const CLAMP_2020_ARG: &str = "2020-01-01T00:00:00Z";
const YEAR_2000_ARG: &str = "2000-01-01T00:00:00Z";
const SOURCE_STORED: StoredDuration = StoredDuration::seconds(1_275_782_400);
const UPDATED_STORED: StoredDuration = StoredDuration::seconds(1_600_000_000);
const MTIME_2001_STORED: StoredDuration = StoredDuration::seconds(978_307_200);
const CTIME_2002_STORED: StoredDuration = StoredDuration::seconds(1_009_843_200);
const ATIME_2003_STORED: StoredDuration = StoredDuration::seconds(1_041_379_200);

type StoredTimes = (
    Option<StoredDuration>,
    Option<StoredDuration>,
    Option<StoredDuration>,
);
type RestoredTimes = (RestoredField, RestoredField);
type ExtractCase = (&'static str, &'static [&'static str], RestoredTimes);

#[derive(Clone, Copy, Debug)]
enum Field {
    Absent,
    Exact(StoredDuration),
    SourceCreated,
}

#[derive(Clone, Copy, Debug)]
struct ExpectedTimes {
    created: Field,
    modified: Field,
    accessed: Field,
}

impl ExpectedTimes {
    fn absent() -> Self {
        Self {
            created: Field::Absent,
            modified: Field::Absent,
            accessed: Field::Absent,
        }
    }

    fn from_source(source: StoredDuration) -> Self {
        Self {
            created: Field::SourceCreated,
            modified: Field::Exact(source),
            accessed: Field::Exact(source),
        }
    }
}

fn assert_field(
    actual: Option<StoredDuration>,
    expected: Field,
    source_created: Option<StoredDuration>,
    context: &str,
) {
    match expected {
        Field::Absent => {
            assert!(
                actual.is_none(),
                "{context}: expected no timestamp, got {actual:?}"
            )
        }
        Field::Exact(expected) => assert_eq!(actual, Some(expected), "{context}"),
        Field::SourceCreated => match source_created {
            Some(expected) => assert_eq!(actual, Some(expected), "{context}"),
            None => assert!(
                actual.is_some(),
                "{context}: expected a timestamp, got none"
            ),
        },
    }
}

fn assert_times(
    actual: StoredTimes,
    expected: ExpectedTimes,
    source_created: Option<StoredDuration>,
    context: &str,
) {
    assert_field(
        actual.0,
        expected.created,
        source_created,
        &format!("{context} created"),
    );
    assert_field(
        actual.1,
        expected.modified,
        source_created,
        &format!("{context} modified"),
    );
    assert_field(
        actual.2,
        expected.accessed,
        source_created,
        &format!("{context} accessed"),
    );
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pna_timestamp_flags_{}_{}",
            std::process::id(),
            suffix
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn case_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn run_success(dir: &Path, args: &[&str]) {
    let output = run(dir, args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_source(dir: &Path, name: &str, content: &[u8], time: Duration) {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    let file = fs::File::options().write(true).open(&path).unwrap();
    file.set_times(
        fs::FileTimes::new()
            .set_modified(UNIX_EPOCH + time)
            .set_accessed(UNIX_EPOCH + time),
    )
    .unwrap();
}

fn source_created(path: &Path) -> Option<StoredDuration> {
    let created = fs::metadata(path).unwrap().created().ok()?;
    let since_epoch = created.duration_since(UNIX_EPOCH).ok()?;
    Some(
        StoredDuration::seconds(since_epoch.as_secs() as i64)
            + StoredDuration::nanoseconds(since_epoch.subsec_nanos().into()),
    )
}

fn entry_times(path: &Path) -> Vec<(String, StoredTimes)> {
    let mut archive = Archive::read_header(fs::File::open(path).unwrap()).unwrap();
    let mut entries = Vec::new();
    for entry in archive.entries() {
        match entry.unwrap() {
            ReadEntry::Normal(entry) => {
                let metadata = entry.metadata();
                entries.push((
                    entry.header().path().to_string(),
                    (metadata.created(), metadata.modified(), metadata.accessed()),
                ));
            }
            ReadEntry::Solid(_) => panic!("expected a regular archive entry"),
        }
    }
    entries
}

fn entries_named<'a>(entries: &'a [(String, StoredTimes)], name: &str) -> Vec<&'a StoredTimes> {
    entries
        .iter()
        .filter(|(path, _)| path.ends_with(name))
        .map(|(_, times)| times)
        .collect()
}

fn flag_cases() -> Vec<(&'static str, Vec<&'static str>, ExpectedTimes)> {
    let source = ExpectedTimes::from_source(SOURCE_STORED);
    let absent = ExpectedTimes::absent();
    vec![
        ("positive", vec!["--preserve-timestamps"], source),
        ("positive-alias", vec!["--keep-timestamp"], source),
        ("negative", vec!["--no-preserve-timestamps"], absent),
        ("negative-alias", vec!["--no-keep-timestamp"], absent),
        (
            "mtime",
            vec!["--mtime", MTIME_2001_ARG],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(SOURCE_STORED),
            },
        ),
        (
            "ctime",
            vec!["--ctime", CTIME_2002_ARG],
            ExpectedTimes {
                created: Field::Exact(CTIME_2002_STORED),
                modified: Field::Exact(SOURCE_STORED),
                accessed: Field::Exact(SOURCE_STORED),
            },
        ),
        (
            "atime",
            vec!["--atime", ATIME_2003_ARG],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(SOURCE_STORED),
                accessed: Field::Exact(ATIME_2003_STORED),
            },
        ),
        (
            "all-overrides",
            vec![
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
            ],
            ExpectedTimes {
                created: Field::Exact(CTIME_2002_STORED),
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(ATIME_2003_STORED),
            },
        ),
        (
            "preserve-with-all-overrides",
            vec![
                "--preserve-timestamps",
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
            ],
            ExpectedTimes {
                created: Field::Exact(CTIME_2002_STORED),
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(ATIME_2003_STORED),
            },
        ),
        (
            "clamp-mtime-before-source",
            vec!["--mtime", MTIME_2001_ARG, "--clamp-mtime"],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(SOURCE_STORED),
            },
        ),
        (
            "clamp-mtime-after-source",
            vec!["--mtime", CLAMP_2020_ARG, "--clamp-mtime"],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(SOURCE_STORED),
                accessed: Field::Exact(SOURCE_STORED),
            },
        ),
        (
            "negative-mtime-clamp",
            vec![
                "--no-preserve-timestamps",
                "--mtime",
                MTIME_2001_ARG,
                "--clamp-mtime",
            ],
            absent,
        ),
        (
            "negative-then-mtime",
            vec!["--no-preserve-timestamps", "--mtime", MTIME_2001_ARG],
            absent,
        ),
        (
            "mtime-then-negative",
            vec!["--mtime", MTIME_2001_ARG, "--no-preserve-timestamps"],
            absent,
        ),
        (
            "negative-then-ctime",
            vec!["--no-preserve-timestamps", "--ctime", CTIME_2002_ARG],
            absent,
        ),
        (
            "negative-then-atime",
            vec!["--no-preserve-timestamps", "--atime", ATIME_2003_ARG],
            absent,
        ),
        (
            "negative-then-all-overrides",
            vec![
                "--no-preserve-timestamps",
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
            ],
            absent,
        ),
        (
            "all-overrides-then-negative",
            vec![
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
                "--no-preserve-timestamps",
            ],
            absent,
        ),
    ]
}

fn bsdtar_negative_cases() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("short-m", vec!["-m"]),
        ("short-m-then-mtime", vec!["-m", "--mtime", MTIME_2001_ARG]),
        ("mtime-then-short-m", vec!["--mtime", MTIME_2001_ARG, "-m"]),
        ("modification-time", vec!["--modification-time"]),
    ]
}

#[test]
fn create_timestamp_flags_preserve_existing_semantics() {
    let dir = TestDirectory::new();
    let families: &[(&str, &[&str], ExpectedTimes)] = &[
        ("create", &["create"], ExpectedTimes::absent()),
        (
            "bsdtar",
            &["compat", "bsdtar", "-c"],
            ExpectedTimes::from_source(SOURCE_STORED),
        ),
    ];
    for (family, command, default_expected) in families {
        let run_case = |case_dir: &Path, flags: &[&str]| -> (StoredTimes, Option<StoredDuration>) {
            write_source(
                case_dir,
                "source.txt",
                b"timestamp regression test",
                SOURCE_TIME,
            );
            let created = source_created(&case_dir.join("source.txt"));
            let mut args = command.to_vec();
            args.extend(["-f", "archive.pna"]);
            args.extend_from_slice(flags);
            args.push("source.txt");
            run_success(case_dir, &args);
            let entries = entry_times(&case_dir.join("archive.pna"));
            let matched = entries_named(&entries, "source.txt");
            assert_eq!(matched.len(), 1);
            run_success(
                case_dir,
                &["extract", "-f", "archive.pna", "--out-dir", "roundtrip"],
            );
            assert_eq!(
                fs::read(case_dir.join("roundtrip/source.txt")).unwrap(),
                b"timestamp regression test",
                "{family:?} roundtrip content"
            );
            (*matched[0], created)
        };

        let default_dir = dir.case_dir(&format!("{family}-default"));
        let (times, created) = run_case(&default_dir, &[]);
        assert_times(
            times,
            *default_expected,
            created,
            &format!("{family} default"),
        );

        for (index, (name, flags, expected)) in flag_cases().iter().enumerate() {
            let case_dir = dir.case_dir(&format!("{family}-{index}"));
            let (times, created) = run_case(&case_dir, flags);
            assert_times(times, *expected, created, &format!("{family} {name}"));
        }
    }

    for (index, (name, flags)) in bsdtar_negative_cases().iter().enumerate() {
        let case_dir = dir.case_dir(&format!("bsdtar-negative-{index}"));
        write_source(
            &case_dir,
            "source.txt",
            b"timestamp regression test",
            SOURCE_TIME,
        );
        let mut args = vec!["compat", "bsdtar", "-c", "-f", "archive.pna"];
        args.extend_from_slice(flags);
        args.push("source.txt");
        run_success(&case_dir, &args);
        let entries = entry_times(&case_dir.join("archive.pna"));
        let matched = entries_named(&entries, "source.txt");
        assert_eq!(matched.len(), 1);
        assert_times(
            *matched[0],
            ExpectedTimes::absent(),
            None,
            &format!("bsdtar {name}"),
        );
    }
}

#[test]
fn append_timestamp_flags_preserve_existing_semantics() {
    let dir = TestDirectory::new();
    let families: &[(&str, &[&str], ExpectedTimes)] = &[
        ("append", &["append"], ExpectedTimes::absent()),
        (
            "bsdtar",
            &["compat", "bsdtar", "-r"],
            ExpectedTimes::from_source(SOURCE_STORED),
        ),
    ];
    for (family, command, default_expected) in families {
        let run_case = |case_dir: &Path, flags: &[&str]| {
            write_source(case_dir, "seed.txt", b"seed", SOURCE_TIME);
            run_success(case_dir, &["create", "-f", "archive.pna", "seed.txt"]);
            write_source(case_dir, "added.txt", b"added", SOURCE_TIME);
            let created = source_created(&case_dir.join("added.txt"));
            let mut args = command.to_vec();
            args.extend(["-f", "archive.pna"]);
            args.extend_from_slice(flags);
            args.push("added.txt");
            run_success(case_dir, &args);
            let entries = entry_times(&case_dir.join("archive.pna"));
            assert_eq!(entries.len(), 2);
            let seed = entries_named(&entries, "seed.txt");
            assert_eq!(seed.len(), 1);
            assert_times(*seed[0], ExpectedTimes::absent(), None, "seed entry");
            let added = entries_named(&entries, "added.txt");
            assert_eq!(added.len(), 1);
            (*added[0], created)
        };

        let default_dir = dir.case_dir(&format!("{family}-default"));
        let (added, created) = run_case(&default_dir, &[]);
        assert_times(
            added,
            *default_expected,
            created,
            &format!("{family} default"),
        );

        for (index, (name, flags, expected)) in flag_cases().iter().enumerate() {
            let case_dir = dir.case_dir(&format!("{family}-{index}"));
            let (added, created) = run_case(&case_dir, flags);
            assert_times(added, *expected, created, &format!("{family} {name}"));
        }
    }

    for (index, (name, flags)) in bsdtar_negative_cases().iter().enumerate() {
        let case_dir = dir.case_dir(&format!("bsdtar-negative-{index}"));
        write_source(&case_dir, "seed.txt", b"seed", SOURCE_TIME);
        run_success(&case_dir, &["create", "-f", "archive.pna", "seed.txt"]);
        write_source(&case_dir, "added.txt", b"added", SOURCE_TIME);
        let mut args = vec!["compat", "bsdtar", "-r", "-f", "archive.pna"];
        args.extend_from_slice(flags);
        args.push("added.txt");
        run_success(&case_dir, &args);
        let entries = entry_times(&case_dir.join("archive.pna"));
        let added = entries_named(&entries, "added.txt");
        assert_eq!(added.len(), 1);
        assert_times(
            *added[0],
            ExpectedTimes::absent(),
            None,
            &format!("bsdtar {name}"),
        );
    }
}

#[test]
fn update_timestamp_flags_preserve_existing_semantics() {
    let dir = TestDirectory::new();
    let updated_source = ExpectedTimes::from_source(UPDATED_STORED);
    let absent = ExpectedTimes::absent();
    let families: &[(&str, &[&str], ExpectedTimes)] = &[
        ("update", &["update"], absent),
        ("bsdtar", &["compat", "bsdtar", "-u"], updated_source),
    ];
    let cases: &[(&str, &[&str], ExpectedTimes)] = &[
        ("preserve", &["--preserve-timestamps"], updated_source),
        ("positive-alias", &["--keep-timestamp"], updated_source),
        ("negative", &["--no-preserve-timestamps"], absent),
        ("negative-alias", &["--no-keep-timestamp"], absent),
        (
            "mtime",
            &["--mtime", MTIME_2001_ARG],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(UPDATED_STORED),
            },
        ),
        (
            "ctime",
            &["--ctime", CTIME_2002_ARG],
            ExpectedTimes {
                created: Field::Exact(CTIME_2002_STORED),
                modified: Field::Exact(UPDATED_STORED),
                accessed: Field::Exact(UPDATED_STORED),
            },
        ),
        (
            "atime",
            &["--atime", ATIME_2003_ARG],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(UPDATED_STORED),
                accessed: Field::Exact(ATIME_2003_STORED),
            },
        ),
        (
            "all-overrides",
            &[
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
            ],
            ExpectedTimes {
                created: Field::Exact(CTIME_2002_STORED),
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(ATIME_2003_STORED),
            },
        ),
        (
            "preserve-with-all-overrides",
            &[
                "--preserve-timestamps",
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
            ],
            ExpectedTimes {
                created: Field::Exact(CTIME_2002_STORED),
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(ATIME_2003_STORED),
            },
        ),
        (
            "clamp-mtime",
            &["--mtime", MTIME_2001_ARG, "--clamp-mtime"],
            ExpectedTimes {
                created: Field::SourceCreated,
                modified: Field::Exact(MTIME_2001_STORED),
                accessed: Field::Exact(UPDATED_STORED),
            },
        ),
        (
            "negative-mtime-clamp",
            &[
                "--no-preserve-timestamps",
                "--mtime",
                MTIME_2001_ARG,
                "--clamp-mtime",
            ],
            absent,
        ),
        (
            "negative-then-mtime",
            &["--no-preserve-timestamps", "--mtime", MTIME_2001_ARG],
            absent,
        ),
        (
            "mtime-then-negative",
            &["--mtime", MTIME_2001_ARG, "--no-preserve-timestamps"],
            absent,
        ),
        (
            "negative-then-ctime",
            &["--no-preserve-timestamps", "--ctime", CTIME_2002_ARG],
            absent,
        ),
        (
            "negative-then-atime",
            &["--no-preserve-timestamps", "--atime", ATIME_2003_ARG],
            absent,
        ),
        (
            "negative-then-all-overrides",
            &[
                "--no-preserve-timestamps",
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
            ],
            absent,
        ),
        (
            "all-overrides-then-negative",
            &[
                "--mtime",
                MTIME_2001_ARG,
                "--ctime",
                CTIME_2002_ARG,
                "--atime",
                ATIME_2003_ARG,
                "--no-preserve-timestamps",
            ],
            absent,
        ),
    ];
    for (family, command, default_expected) in families {
        let run_case = |case_dir: &Path, flags: &[&str]| -> (StoredTimes, Option<StoredDuration>) {
            write_source(case_dir, "source.txt", b"v1", SOURCE_TIME);
            let original_created = source_created(&case_dir.join("source.txt"));
            run_success(
                case_dir,
                &[
                    "create",
                    "-f",
                    "archive.pna",
                    "--preserve-timestamps",
                    "source.txt",
                ],
            );
            write_source(
                case_dir,
                "source.txt",
                b"v2 with different content",
                UPDATED_TIME,
            );
            let revised_created = source_created(&case_dir.join("source.txt"));
            let mut args = command.to_vec();
            args.extend(["-f", "archive.pna"]);
            args.extend_from_slice(flags);
            args.push("source.txt");
            run_success(case_dir, &args);
            let entries = entry_times(&case_dir.join("archive.pna"));
            let matched = entries_named(&entries, "source.txt");
            assert_eq!(matched.len(), 2, "update appends a revised entry");
            assert_times(
                *matched[0],
                ExpectedTimes::from_source(SOURCE_STORED),
                original_created,
                "original entry",
            );
            (*matched[1], revised_created)
        };

        let default_dir = dir.case_dir(&format!("{family}-default"));
        let (revised, created) = run_case(&default_dir, &[]);
        assert_times(
            revised,
            *default_expected,
            created,
            &format!("{family} default"),
        );

        for (index, (name, flags, expected)) in cases.iter().enumerate() {
            let case_dir = dir.case_dir(&format!("{family}-{index}"));
            let (revised, created) = run_case(&case_dir, flags);
            assert_times(revised, *expected, created, &format!("{family} {name}"));
        }

        let case_dir = dir.case_dir(&format!("{family}-untimestamped-original"));
        write_source(&case_dir, "source.txt", b"v1", SOURCE_TIME);
        run_success(&case_dir, &["create", "-f", "archive.pna", "source.txt"]);
        write_source(
            &case_dir,
            "source.txt",
            b"v2 with different content",
            UPDATED_TIME,
        );
        let revised_created = source_created(&case_dir.join("source.txt"));
        let mut args = command.to_vec();
        args.extend(["-f", "archive.pna", "--preserve-timestamps", "source.txt"]);
        run_success(&case_dir, &args);
        let entries = entry_times(&case_dir.join("archive.pna"));
        let matched = entries_named(&entries, "source.txt");
        assert_eq!(matched.len(), 2);
        assert_times(*matched[0], absent, None, "untimestamped original entry");
        assert_times(
            *matched[1],
            updated_source,
            revised_created,
            "revised entry",
        );
    }

    for (index, (name, flags)) in bsdtar_negative_cases().iter().enumerate() {
        let case_dir = dir.case_dir(&format!("bsdtar-negative-{index}"));
        write_source(&case_dir, "source.txt", b"v1", SOURCE_TIME);
        run_success(
            &case_dir,
            &[
                "create",
                "-f",
                "archive.pna",
                "--preserve-timestamps",
                "source.txt",
            ],
        );
        write_source(
            &case_dir,
            "source.txt",
            b"v2 with different content",
            UPDATED_TIME,
        );
        let mut args = vec!["compat", "bsdtar", "-u", "-f", "archive.pna"];
        args.extend_from_slice(flags);
        args.push("source.txt");
        run_success(&case_dir, &args);
        let entries = entry_times(&case_dir.join("archive.pna"));
        let matched = entries_named(&entries, "source.txt");
        assert_eq!(matched.len(), 2);
        assert_times(
            *matched[1],
            absent,
            None,
            &format!("bsdtar {name} revised entry"),
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum RestoredField {
    Stored(Duration),
    NotStored,
}

fn assert_restored(actual: SystemTime, expected: RestoredField, context: &str) {
    match expected {
        RestoredField::Stored(expected) => {
            assert_eq!(actual, UNIX_EPOCH + expected, "{context}");
        }
        RestoredField::NotStored => {
            assert!(
                actual > UNIX_EPOCH + UPDATED_TIME,
                "{context}: expected extraction time, got {actual:?}"
            );
        }
    }
}

#[test]
fn extract_timestamp_flags_preserve_existing_semantics() {
    let dir = TestDirectory::new();
    let fixture_dir = dir.case_dir("fixture");
    write_source(
        &fixture_dir,
        "source.txt",
        b"timestamp regression test",
        SOURCE_TIME,
    );
    run_success(
        &fixture_dir,
        &[
            "create",
            "-f",
            "archive.pna",
            "--preserve-timestamps",
            "--mtime",
            MTIME_2001_ARG,
            "--ctime",
            CTIME_2002_ARG,
            "--atime",
            ATIME_2003_ARG,
            "source.txt",
        ],
    );
    let fixture = fixture_dir.join("archive.pna");
    let fixture_entries = entry_times(&fixture);
    let fixture_matched = entries_named(&fixture_entries, "source.txt");
    assert_eq!(fixture_matched.len(), 1);
    assert_times(
        *fixture_matched[0],
        ExpectedTimes {
            created: Field::Exact(CTIME_2002_STORED),
            modified: Field::Exact(MTIME_2001_STORED),
            accessed: Field::Exact(ATIME_2003_STORED),
        },
        None,
        "fixture entry",
    );

    let stored: RestoredTimes = (
        RestoredField::Stored(MTIME_2001),
        RestoredField::Stored(ATIME_2003),
    );
    let not_stored: RestoredTimes = (RestoredField::NotStored, RestoredField::NotStored);
    let cases: &[ExtractCase] = &[
        ("preserve", &["--preserve-timestamps"], stored),
        ("positive-alias", &["--keep-timestamp"], stored),
        ("negative", &["--no-preserve-timestamps"], not_stored),
        ("negative-alias", &["--no-keep-timestamp"], not_stored),
        (
            "mtime",
            &["--mtime", CTIME_2002_ARG],
            (
                RestoredField::Stored(CTIME_2002),
                RestoredField::Stored(ATIME_2003),
            ),
        ),
        ("ctime", &["--ctime", CTIME_2002_ARG], stored),
        (
            "atime",
            &["--atime", CTIME_2002_ARG],
            (
                RestoredField::Stored(MTIME_2001),
                RestoredField::Stored(CTIME_2002),
            ),
        ),
        (
            "all-overrides",
            &[
                "--mtime",
                CTIME_2002_ARG,
                "--ctime",
                ATIME_2003_ARG,
                "--atime",
                CTIME_2002_ARG,
            ],
            (
                RestoredField::Stored(CTIME_2002),
                RestoredField::Stored(CTIME_2002),
            ),
        ),
        (
            "preserve-with-all-overrides",
            &[
                "--preserve-timestamps",
                "--mtime",
                CTIME_2002_ARG,
                "--ctime",
                ATIME_2003_ARG,
                "--atime",
                CTIME_2002_ARG,
            ],
            (
                RestoredField::Stored(CTIME_2002),
                RestoredField::Stored(CTIME_2002),
            ),
        ),
        (
            "clamp-mtime-above-stored",
            &["--mtime", CTIME_2002_ARG, "--clamp-mtime"],
            stored,
        ),
        (
            "clamp-mtime-below-stored",
            &["--mtime", YEAR_2000_ARG, "--clamp-mtime"],
            (
                RestoredField::Stored(YEAR_2000),
                RestoredField::Stored(ATIME_2003),
            ),
        ),
        (
            "negative-mtime-clamp",
            &[
                "--no-preserve-timestamps",
                "--mtime",
                CTIME_2002_ARG,
                "--clamp-mtime",
            ],
            not_stored,
        ),
        (
            "negative-then-mtime",
            &["--no-preserve-timestamps", "--mtime", CTIME_2002_ARG],
            not_stored,
        ),
        (
            "mtime-then-negative",
            &["--mtime", CTIME_2002_ARG, "--no-preserve-timestamps"],
            not_stored,
        ),
        (
            "negative-then-ctime",
            &["--no-preserve-timestamps", "--ctime", CTIME_2002_ARG],
            not_stored,
        ),
        (
            "negative-then-atime",
            &["--no-preserve-timestamps", "--atime", CTIME_2002_ARG],
            not_stored,
        ),
        (
            "negative-then-all-overrides",
            &[
                "--no-preserve-timestamps",
                "--mtime",
                CTIME_2002_ARG,
                "--ctime",
                ATIME_2003_ARG,
                "--atime",
                CTIME_2002_ARG,
            ],
            not_stored,
        ),
        (
            "all-overrides-then-negative",
            &[
                "--mtime",
                CTIME_2002_ARG,
                "--ctime",
                ATIME_2003_ARG,
                "--atime",
                CTIME_2002_ARG,
                "--no-preserve-timestamps",
            ],
            not_stored,
        ),
    ];
    let families: &[(&str, &[&str], RestoredTimes)] = &[
        ("extract", &["extract"], not_stored),
        ("bsdtar", &["compat", "bsdtar", "-x"], stored),
    ];
    for (family, command, default_expected) in families {
        let run_case = |case_dir: &Path, flags: &[&str]| -> (SystemTime, SystemTime) {
            fs::copy(&fixture, case_dir.join("archive.pna")).unwrap();
            fs::create_dir(case_dir.join("out")).unwrap();
            let mut args = command.to_vec();
            args.extend(["-f", "archive.pna"]);
            args.extend_from_slice(flags);
            if *family == "extract" {
                args.extend(["--out-dir", "out"]);
            } else {
                args.extend(["-C", "out"]);
            }
            run_success(case_dir, &args);
            let metadata = fs::metadata(case_dir.join("out/source.txt")).unwrap();
            (metadata.modified().unwrap(), metadata.accessed().unwrap())
        };

        let default_dir = dir.case_dir(&format!("{family}-default"));
        let (modified, accessed) = run_case(&default_dir, &[]);
        assert_restored(
            modified,
            default_expected.0,
            &format!("{family} default modified"),
        );
        assert_restored(
            accessed,
            default_expected.1,
            &format!("{family} default accessed"),
        );

        for (index, (name, flags, expected)) in cases.iter().enumerate() {
            let case_dir = dir.case_dir(&format!("{family}-{index}"));
            let (modified, accessed) = run_case(&case_dir, flags);
            assert_restored(modified, expected.0, &format!("{family} {name} modified"));
            assert_restored(accessed, expected.1, &format!("{family} {name} accessed"));
        }
    }

    for (index, (name, flags)) in bsdtar_negative_cases().iter().enumerate() {
        let case_dir = dir.case_dir(&format!("bsdtar-negative-{index}"));
        fs::copy(&fixture, case_dir.join("archive.pna")).unwrap();
        fs::create_dir(case_dir.join("out")).unwrap();
        let mut args = vec!["compat", "bsdtar", "-x", "-f", "archive.pna", "-C", "out"];
        args.extend_from_slice(flags);
        run_success(&case_dir, &args);
        let metadata = fs::metadata(case_dir.join("out/source.txt")).unwrap();
        assert_restored(
            metadata.modified().unwrap(),
            RestoredField::NotStored,
            &format!("bsdtar {name} modified"),
        );
        assert_restored(
            metadata.accessed().unwrap(),
            RestoredField::NotStored,
            &format!("bsdtar {name} accessed"),
        );
    }
}

#[test]
fn positive_and_negative_timestamp_flags_conflict() {
    let dir = TestDirectory::new();
    let orders: &[&[&str]] = &[
        &["--preserve-timestamps", "--no-preserve-timestamps"],
        &["--no-preserve-timestamps", "--preserve-timestamps"],
    ];

    for (index, order) in orders.iter().enumerate() {
        let case_dir = dir.case_dir(&format!("create-{index}"));
        write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
        let mut args = vec!["create", "-f", "archive.pna"];
        args.extend_from_slice(order);
        args.push("source.txt");
        let output = run(&case_dir, &args);
        assert!(!output.status.success());
        assert!(!case_dir.join("archive.pna").exists());

        let case_dir = dir.case_dir(&format!("bsdtar-create-{index}"));
        write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
        let mut args = vec!["compat", "bsdtar", "-c", "-f", "archive.pna"];
        args.extend_from_slice(order);
        args.push("source.txt");
        let output = run(&case_dir, &args);
        assert!(!output.status.success());
        assert!(!case_dir.join("archive.pna").exists());
    }

    for (name, command) in [
        ("append", &["append"][..]),
        ("update", &["update"][..]),
        ("bsdtar-append", &["compat", "bsdtar", "-r"][..]),
        ("bsdtar-update", &["compat", "bsdtar", "-u"][..]),
    ] {
        for (index, order) in orders.iter().enumerate() {
            let case_dir = dir.case_dir(&format!("{name}-{index}"));
            write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
            run_success(&case_dir, &["create", "-f", "archive.pna", "source.txt"]);
            let before = fs::read(case_dir.join("archive.pna")).unwrap();
            let mut args = command.to_vec();
            args.extend(["-f", "archive.pna"]);
            args.extend_from_slice(order);
            args.push("source.txt");
            let output = run(&case_dir, &args);
            assert!(!output.status.success(), "{name} {order:?}");
            assert_eq!(
                fs::read(case_dir.join("archive.pna")).unwrap(),
                before,
                "{name}"
            );
        }
    }

    for (name, command) in [
        ("extract", &["extract"][..]),
        ("bsdtar-extract", &["compat", "bsdtar", "-x"][..]),
    ] {
        for (index, order) in orders.iter().enumerate() {
            let case_dir = dir.case_dir(&format!("{name}-{index}"));
            write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
            run_success(&case_dir, &["create", "-f", "archive.pna", "source.txt"]);
            fs::create_dir(case_dir.join("out")).unwrap();
            let mut args = command.to_vec();
            args.extend(["-f", "archive.pna"]);
            args.extend_from_slice(order);
            if name == "extract" {
                args.extend(["--out-dir", "out"]);
            } else {
                args.extend(["-C", "out"]);
            }
            let output = run(&case_dir, &args);
            assert!(!output.status.success(), "{name} {order:?}");
            assert!(!case_dir.join("out/source.txt").exists(), "{name}");
        }
    }

    let case_dir = dir.case_dir("bsdtar-create-short-m");
    write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
    let output = run(
        &case_dir,
        &[
            "compat",
            "bsdtar",
            "-c",
            "-f",
            "archive.pna",
            "-m",
            "--preserve-timestamps",
            "source.txt",
        ],
    );
    assert!(!output.status.success());
    assert!(!case_dir.join("archive.pna").exists());

    for (name, command) in [
        ("bsdtar-append-short-m", &["compat", "bsdtar", "-r"][..]),
        ("bsdtar-update-short-m", &["compat", "bsdtar", "-u"][..]),
    ] {
        let case_dir = dir.case_dir(name);
        write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
        run_success(&case_dir, &["create", "-f", "archive.pna", "source.txt"]);
        let before = fs::read(case_dir.join("archive.pna")).unwrap();
        let mut args = command.to_vec();
        args.extend([
            "-f",
            "archive.pna",
            "-m",
            "--preserve-timestamps",
            "source.txt",
        ]);
        let output = run(&case_dir, &args);
        assert!(!output.status.success(), "{name}");
        assert_eq!(
            fs::read(case_dir.join("archive.pna")).unwrap(),
            before,
            "{name}"
        );
    }

    let case_dir = dir.case_dir("bsdtar-extract-short-m");
    write_source(&case_dir, "source.txt", b"content", SOURCE_TIME);
    run_success(&case_dir, &["create", "-f", "archive.pna", "source.txt"]);
    fs::create_dir(case_dir.join("out")).unwrap();
    let output = run(
        &case_dir,
        &[
            "compat",
            "bsdtar",
            "-x",
            "-f",
            "archive.pna",
            "-C",
            "out",
            "-m",
            "--preserve-timestamps",
        ],
    );
    assert!(!output.status.success());
    assert!(!case_dir.join("out/source.txt").exists());
}
