use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    collections::HashSet,
    fs, thread,
    time::{Duration, SystemTime},
};

/// Precondition: Archive contains `file_to_update`. Prepare a reference file whose ctime is *after*
/// the rewritten file so `file_to_update` remains older, plus a newer file that should be skipped.
/// Action: Run `pna experimental update` with `--older-ctime-than reference.txt`.
/// Expectation: Only files whose ctime <= reference (i.e., `file_to_update`) are processed; newer files are ignored.
#[test]
fn update_with_older_ctime_than() {
    setup();
    let base_dir = "update_older_ctime_than";
    let archive_path = format!("{base_dir}/test.pna");
    let file_to_update = format!("{base_dir}/file_to_update.txt");
    let file_to_skip = format!("{base_dir}/file_to_skip.txt");
    let reference_file = format!("{base_dir}/reference.txt");

    fs::create_dir_all(base_dir).unwrap();
    fs::write(&file_to_update, "initial content").unwrap();

    if fs::metadata(&file_to_update).unwrap().created().is_err() {
        eprintln!("Skipping test: creation time (birth time) not supported on this filesystem");
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        &archive_path,
        "--overwrite",
        &file_to_update,
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Recreate the file with updated content *before* creating the reference so its ctime stays older.
    thread::sleep(Duration::from_millis(10));
    fs::write(&file_to_update, "updated content").unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&reference_file, "reference marker").unwrap();
    let reference_ctime = fs::metadata(&reference_file).unwrap().created().unwrap();

    thread::sleep(Duration::from_millis(10));
    fs::write(&file_to_skip, "skip content").unwrap();

    if !confirm_ctime_not_newer_than(&file_to_update, reference_ctime)
        || !wait_until_ctime_newer_than(&file_to_skip, reference_ctime)
    {
        eprintln!(
            "Skipping test: unable to create deterministic creation times on this filesystem"
        );
        return;
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "--file",
        &archive_path,
        &file_to_update,
        &file_to_skip,
        "--unstable",
        "--older-ctime-than",
        &reference_file,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(&archive_path, |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    assert!(
        !seen.contains(&file_to_skip),
        "file newer than reference should not have been added: {file_to_skip}"
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--file",
        &archive_path,
        "--out-dir",
        &format!("{base_dir}/out"),
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let updated_content = fs::read_to_string(format!("{base_dir}/out/{file_to_update}")).unwrap();
    assert_eq!(updated_content, "updated content");
    assert!(
        !fs::metadata(format!("{base_dir}/out/{file_to_skip}")).is_ok(),
        "skip file should not have been extracted/added"
    );
}

fn wait_until_ctime_newer_than(path: &str, baseline: SystemTime) -> bool {
    const MAX_ATTEMPTS: usize = 500;
    const SLEEP_MS: u64 = 10;
    for _ in 0..MAX_ATTEMPTS {
        if fs::metadata(path)
            .ok()
            .and_then(|meta| meta.created().ok())
            .map(|ctime| ctime > baseline)
            .unwrap_or(false)
        {
            return true;
        }
        thread::sleep(Duration::from_millis(SLEEP_MS));
    }
    false
}

fn confirm_ctime_not_newer_than(path: &str, baseline: SystemTime) -> bool {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.created().ok())
        .map(|ctime| ctime <= baseline)
        .unwrap_or(false)
}
