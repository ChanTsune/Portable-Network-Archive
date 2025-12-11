use crate::utils::{archive::for_each_entry, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn create_test_archive(path: &Path, entries: &[(&str, &str)]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    for (name, contents) in entries {
        writer
            .add_entry({
                let mut builder =
                    EntryBuilder::new_file((*name).into(), WriteOptions::builder().build())
                        .unwrap();
                builder.write_all(contents.as_bytes()).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
    }
    writer.finalize().unwrap();
}

fn get_archive_entry_names(path: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for_each_entry(path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names
}

/// Test basic @archive inclusion: create a new archive including entries from an existing archive.
#[test]
fn stdio_archive_inclusion_basic() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_basic");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with some files
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[
            ("old_file1.txt", "old content 1"),
            ("old_file2.txt", "old content 2"),
        ],
    );

    // Create new file to include
    let new_file = base.join("new_file.txt");
    fs::write(&new_file, "new content").unwrap();

    // Create archive including @source.pna and new_file.txt
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "new_file.txt",
            &format!("@{}", source_archive.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Verify the output archive contains all entries
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("new_file.txt"), "Missing new_file.txt");
    assert!(
        entry_names.contains("old_file1.txt"),
        "Missing old_file1.txt"
    );
    assert!(
        entry_names.contains("old_file2.txt"),
        "Missing old_file2.txt"
    );
    assert_eq!(entry_names.len(), 3);
}

/// Test multiple @archive inclusions from different source archives.
#[test]
fn stdio_archive_inclusion_multiple() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_multiple");
    fs::create_dir_all(&base).unwrap();

    // Create first source archive
    let source1 = base.join("source1.pna");
    create_test_archive(&source1, &[("from_source1.txt", "content 1")]);

    // Create second source archive
    let source2 = base.join("source2.pna");
    create_test_archive(&source2, &[("from_source2.txt", "content 2")]);

    // Create new file
    let new_file = base.join("new.txt");
    fs::write(&new_file, "new").unwrap();

    // Create archive including both source archives
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "new.txt",
            &format!("@{}", source1.to_str().unwrap()),
            &format!("@{}", source2.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Verify all entries present
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("new.txt"));
    assert!(entry_names.contains("from_source1.txt"));
    assert!(entry_names.contains("from_source2.txt"));
    assert_eq!(entry_names.len(), 3);
}

/// Test @archive inclusion with solid mode enabled.
#[test]
fn stdio_archive_inclusion_solid() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_solid");
    fs::create_dir_all(&base).unwrap();

    // Create source archive
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[("source_file.txt", "source content here")],
    );

    // Create new file
    let new_file = base.join("new_file.txt");
    fs::write(&new_file, "new content here").unwrap();

    // Create archive with --solid
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "--solid",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "new_file.txt",
            &format!("@{}", source_archive.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Verify entries in output archive
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("new_file.txt"));
    assert!(entry_names.contains("source_file.txt"));
    assert_eq!(entry_names.len(), 2);

    // Verify extraction works
    let extract_dir = base.join("extract");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "--out-dir",
            extract_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(extract_dir.join("new_file.txt")).unwrap(),
        "new content here"
    );
    assert_eq!(
        fs::read_to_string(extract_dir.join("source_file.txt")).unwrap(),
        "source content here"
    );
}

/// Test @archive inclusion in append mode.
#[test]
fn stdio_archive_inclusion_append() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_append");
    fs::create_dir_all(&base).unwrap();

    // Create initial archive
    let archive = base.join("archive.pna");
    create_test_archive(&archive, &[("initial.txt", "initial content")]);

    // Create source archive to include
    let source_archive = base.join("source.pna");
    create_test_archive(&source_archive, &[("from_source.txt", "source content")]);

    // Append entries from source archive
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--append",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            &format!("@{}", source_archive.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Verify all entries present
    let entry_names: HashSet<String> = get_archive_entry_names(&archive).into_iter().collect();
    assert!(entry_names.contains("initial.txt"));
    assert!(entry_names.contains("from_source.txt"));
    assert_eq!(entry_names.len(), 2);
}

/// Test that @archive with non-existent file produces an error.
#[test]
fn stdio_archive_inclusion_nonexistent() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_nonexistent");
    fs::create_dir_all(&base).unwrap();

    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "@nonexistent_archive.pna",
        ])
        .assert()
        .failure();
}

/// Test that creating archive with @- when also outputting to stdout produces an error.
#[test]
fn stdio_archive_inclusion_stdin_stdout_conflict() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_stdin_stdout_conflict");
    fs::create_dir_all(&base).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "-f",
            "-",
            "@-",
        ])
        .assert()
        .failure();
}

/// Test @archive inclusion preserves entry data correctly.
#[test]
fn stdio_archive_inclusion_data_integrity() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_data_integrity");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with larger content
    let source_archive = base.join("source.pna");
    let large_content = "x".repeat(10000);
    create_test_archive(&source_archive, &[("large_file.txt", &large_content)]);

    // Create archive including source
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            &format!("@{}", source_archive.to_str().unwrap()),
        ])
        .assert()
        .success();

    // Extract and verify content
    let extract_dir = base.join("extract");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "--out-dir",
            extract_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(extract_dir.join("large_file.txt")).unwrap(),
        large_content
    );
}

/// Test that entry ordering is preserved when interleaving filesystem paths with @archive inclusions.
/// This verifies the fix for the rayon parallel processing ordering issue.
#[test]
fn stdio_archive_inclusion_ordering() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_ordering");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with entries
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[
            ("from_archive_1.txt", "archive content 1"),
            ("from_archive_2.txt", "archive content 2"),
        ],
    );

    // Create filesystem files
    fs::write(base.join("file_a.txt"), "content a").unwrap();
    fs::write(base.join("file_b.txt"), "content b").unwrap();
    fs::write(base.join("file_c.txt"), "content c").unwrap();

    // Create archive with interleaved filesystem paths and @archive inclusion
    // Order: file_a, @archive, file_b, file_c
    let output_archive = base.join("output.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "file_a.txt",
            &format!("@{}", source_archive.to_str().unwrap()),
            "file_b.txt",
            "file_c.txt",
        ])
        .assert()
        .success();

    // Verify entry ordering in the output archive
    let entry_names = get_archive_entry_names(&output_archive);

    // Expected order: file_a, from_archive_1, from_archive_2, file_b, file_c
    assert_eq!(entry_names.len(), 5, "Expected 5 entries");
    assert_eq!(entry_names[0], "file_a.txt", "First entry should be file_a.txt");
    assert_eq!(
        entry_names[1], "from_archive_1.txt",
        "Second entry should be from_archive_1.txt"
    );
    assert_eq!(
        entry_names[2], "from_archive_2.txt",
        "Third entry should be from_archive_2.txt"
    );
    assert_eq!(entry_names[3], "file_b.txt", "Fourth entry should be file_b.txt");
    assert_eq!(entry_names[4], "file_c.txt", "Fifth entry should be file_c.txt");

    // Verify extraction produces correct content
    let extract_dir = base.join("extract");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--extract",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "--out-dir",
            extract_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(extract_dir.join("file_a.txt")).unwrap(),
        "content a"
    );
    assert_eq!(
        fs::read_to_string(extract_dir.join("file_b.txt")).unwrap(),
        "content b"
    );
    assert_eq!(
        fs::read_to_string(extract_dir.join("file_c.txt")).unwrap(),
        "content c"
    );
    assert_eq!(
        fs::read_to_string(extract_dir.join("from_archive_1.txt")).unwrap(),
        "archive content 1"
    );
    assert_eq!(
        fs::read_to_string(extract_dir.join("from_archive_2.txt")).unwrap(),
        "archive content 2"
    );
}
