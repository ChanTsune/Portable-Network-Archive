//! Freeze the observable timestamp flag semantics before changing clap resolution.

#![cfg(not(target_family = "wasm"))]

use clap::{CommandFactory, Parser, error::ErrorKind};
use pna::{Archive, ReadEntry};
use portable_network_archive::cli::Cli;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

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
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(args)
        .output()
        .unwrap()
}

fn archive_has_modified_time(path: &std::path::Path) -> bool {
    let mut archive = Archive::read_header(fs::File::open(path).unwrap()).unwrap();
    match archive.entries().next().unwrap().unwrap() {
        ReadEntry::Normal(entry) => entry.metadata().modified().is_some(),
        ReadEntry::Solid(_) => panic!("expected a regular archive entry"),
    }
}

#[test]
fn create_timestamp_flags_preserve_existing_semantics() {
    let dir = TestDirectory::new();
    let input = dir.0.join("source.txt");
    fs::write(&input, b"timestamp regression test").unwrap();
    let input = input.to_str().unwrap();
    let timestamp = "2001-01-01T00:00:00Z";

    let cases: &[(&str, &[&str], bool)] = &[
        ("default", &[], false),
        ("positive", &["--preserve-timestamps"], true),
        ("positive-alias", &["--keep-timestamp"], true),
        ("negative", &["--no-preserve-timestamps"], false),
        ("negative-alias", &["--no-keep-timestamp"], false),
        ("mtime", &["--mtime", timestamp], true),
        ("ctime", &["--ctime", timestamp], true),
        ("atime", &["--atime", timestamp], true),
        ("negative-then-mtime", &["--no-preserve-timestamps", "--mtime", timestamp], false),
        ("mtime-then-negative", &["--mtime", timestamp, "--no-preserve-timestamps"], false),
        ("negative-then-ctime", &["--no-preserve-timestamps", "--ctime", timestamp], false),
        ("negative-then-atime", &["--no-preserve-timestamps", "--atime", timestamp], false),
    ];

    for (command, default_preserve) in [
        (&["create"][..], false),
        (&["compat", "bsdtar", "-c"][..], true),
    ] {
        for (index, (name, flags, expected)) in cases.iter().enumerate() {
            let archive_path = dir.0.join(format!(
                "{}-{index}.pna",
                if default_preserve { "bsdtar" } else { "create" }
            ));
            let archive_file = archive_path.to_str().unwrap();
            let mut args = command.to_vec();
            args.extend(["-f", archive_file]);
            args.extend_from_slice(flags);
            args.push(input);

            let output = run(&args);
            assert!(
                output.status.success(),
                "{command:?} {name}: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            assert_eq!(
                archive_has_modified_time(&archive_path),
                if *name == "default" { default_preserve } else { *expected },
                "{command:?} {name}: unexpected timestamp storage"
            );
        }

        // bsdtar's -m is a compatibility spelling of --no-preserve-timestamps.
        if default_preserve {
            let archive_path = dir.0.join("bsdtar-short-m.pna");
            let mut args = command.to_vec();
            args.extend(["-f", archive_path.to_str().unwrap(), "-m", input]);
            let output = run(&args);
            assert!(
                output.status.success(),
                "-m: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(!archive_has_modified_time(&archive_path));
        }
    }
}

#[test]
fn positive_and_negative_timestamp_flags_remain_mutually_exclusive() {
    let prefixes: &[&[&str]] = &[
        &["pna", "create", "-f", "archive.pna", "source.txt"],
        &["pna", "append", "-f", "archive.pna", "source.txt"],
        &["pna", "update", "-f", "archive.pna", "source.txt"],
        &["pna", "extract", "-f", "archive.pna"],
        &["pna", "compat", "bsdtar", "-c", "-f", "archive.pna", "source.txt"],
    ];
    for prefix in prefixes {
        let mut args = prefix.to_vec();
        args.extend(["--preserve-timestamps", "--no-preserve-timestamps"]);
        let err = Cli::try_parse_from(args).unwrap_err();
        assert_eq!(
            err.kind(),
            ErrorKind::ArgumentConflict,
            "{prefix:?}: unexpected error: {err}"
        );
    }
}

#[test]
fn clap_resolves_timestamp_flags_for_every_command() {
    let timestamp = "2001-01-01T00:00:00Z";
    let commands: &[(&[&str], bool)] = &[
        (&["pna", "create", "-f", "archive.pna", "source.txt"], false),
        (&["pna", "append", "-f", "archive.pna", "source.txt"], false),
        (&["pna", "update", "-f", "archive.pna", "source.txt"], false),
        (&["pna", "extract", "-f", "archive.pna"], false),
        (&["pna", "compat", "bsdtar", "-c", "-f", "archive.pna", "source.txt"], true),
    ];
    let cases: &[(&[&str], Option<bool>)] = &[
        (&[], None),
        (&["--preserve-timestamps"], Some(true)),
        (&["--no-preserve-timestamps"], Some(false)),
        (&["--no-keep-timestamp"], Some(false)),
        (&["--mtime", timestamp], Some(true)),
        (&["--ctime", timestamp], Some(true)),
        (&["--atime", timestamp], Some(true)),
        (&["--no-preserve-timestamps", "--mtime", timestamp], Some(false)),
        (&["--mtime", timestamp, "--no-preserve-timestamps"], Some(false)),
        (&["--no-preserve-timestamps", "--ctime", timestamp], Some(false)),
        (&["--no-preserve-timestamps", "--atime", timestamp], Some(false)),
    ];

    for (prefix, default_preserve) in commands {
        for (flags, expected) in cases {
            let mut args = prefix.to_vec();
            args.extend_from_slice(flags);
            let matches = Cli::command().try_get_matches_from(args).unwrap();
            let (_, subcommand) = matches.subcommand().unwrap();
            let command_matches = if prefix[1] == "compat" {
                subcommand.subcommand().unwrap().1
            } else {
                subcommand
            };
            assert_eq!(
                command_matches.get_one::<bool>("keep_timestamp").copied(),
                Some(expected.unwrap_or(*default_preserve)),
                "{prefix:?}, {flags:?}"
            );
        }
    }
}
