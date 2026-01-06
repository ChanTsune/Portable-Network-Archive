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
    // Note: @source.pna is relative to -C directory, just like other positional arguments
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
            "@source.pna",
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
            "@source1.pna",
            "@source2.pna",
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
            "@source.pna",
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
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
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
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
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

/// Precondition: Source archive contains files with different extensions.
/// Action: Create archive with `--exclude` pattern and `@archive` inclusion.
/// Expectation: Entries matching the exclude pattern are filtered out from the included archive.
#[test]
fn stdio_archive_inclusion_exclude_filter() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_exclude_filter");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with various file types
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[
            ("keep.txt", "keep this"),
            ("exclude.log", "exclude this"),
            ("also_keep.txt", "also keep"),
            ("also_exclude.log", "also exclude"),
        ],
    );

    // Create archive with --exclude='*.log'
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
            "--exclude=*.log",
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
        ])
        .assert()
        .success();

    // Verify only .txt files are included
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("keep.txt"), "Missing keep.txt");
    assert!(
        entry_names.contains("also_keep.txt"),
        "Missing also_keep.txt"
    );
    assert!(
        !entry_names.contains("exclude.log"),
        "exclude.log should be filtered"
    );
    assert!(
        !entry_names.contains("also_exclude.log"),
        "also_exclude.log should be filtered"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: Source archive contains files with various names.
/// Action: Create archive with `--include` pattern and `@archive` inclusion.
/// Expectation: Only entries matching the include pattern are included from the source archive.
#[test]
fn stdio_archive_inclusion_include_filter() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_include_filter");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with various files
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[
            ("foo.txt", "foo content"),
            ("bar.txt", "bar content"),
            ("foobar.txt", "foobar content"),
            ("baz.txt", "baz content"),
        ],
    );

    // Create archive with --include='*foo*'
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
            "--include=*foo*",
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
        ])
        .assert()
        .success();

    // Verify only files matching *foo* are included
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("foo.txt"), "Missing foo.txt");
    assert!(entry_names.contains("foobar.txt"), "Missing foobar.txt");
    assert!(
        !entry_names.contains("bar.txt"),
        "bar.txt should be filtered"
    );
    assert!(
        !entry_names.contains("baz.txt"),
        "baz.txt should be filtered"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: Source archive contains entries; filesystem files exist.
/// Action: Create archive with `@archive` as the first argument, followed by filesystem files.
/// Expectation: Entry order is preserved: archive entries first, then filesystem files.
#[test]
fn stdio_archive_inclusion_archive_first() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_archive_first");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with entries
    let source_archive = base.join("source.pna");
    create_test_archive(&source_archive, &[("from_archive.txt", "archive content")]);

    // Create filesystem file
    fs::write(base.join("after.txt"), "after content").unwrap();

    // Create archive with @archive FIRST, then filesystem file
    // Note: -C affects all subsequent positional arguments including @archive
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
            "@source.pna",
            "after.txt",
        ])
        .assert()
        .success();

    // Verify entry ordering: archive entry first, then filesystem file
    let entry_names = get_archive_entry_names(&output_archive);
    assert_eq!(entry_names.len(), 2, "Expected 2 entries");
    assert_eq!(
        entry_names[0], "from_archive.txt",
        "First entry should be from_archive.txt (from @archive)"
    );
    assert_eq!(
        entry_names[1], "after.txt",
        "Second entry should be after.txt (filesystem file)"
    );
}

/// Precondition: Source archive contains zero entries.
/// Action: Create archive with empty `@archive` inclusion and a filesystem file.
/// Expectation: Output archive contains only the filesystem file; no error occurs.
#[test]
fn stdio_archive_inclusion_empty_archive() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_empty_archive");
    fs::create_dir_all(&base).unwrap();

    // Create empty source archive (no entries)
    let source_archive = base.join("empty.pna");
    create_test_archive(&source_archive, &[]);

    // Create filesystem file
    fs::write(base.join("file.txt"), "content").unwrap();

    // Create archive including empty @archive and a file
    // -C is placed before @empty.pna so the archive path is relative to base
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
            "@empty.pna",
            "file.txt",
        ])
        .assert()
        .success();

    // Verify output contains only the filesystem file
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("file.txt"), "Missing file.txt");
    assert_eq!(entry_names.len(), 1, "Should contain exactly 1 entry");
}

/// Precondition: Source archive contains files matching various patterns.
/// Action: Create archive with both `--include` and `--exclude` patterns and `@archive` inclusion.
/// Expectation: Exclude takes precedence; only entries matching include but not exclude are included.
#[test]
fn stdio_archive_inclusion_combined_filters() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_combined_filters");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with files that match different filter combinations
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[
            ("foo.txt", "matches include, not excluded"),
            ("bar.txt", "does not match include"),
            ("foobar.log", "matches include, but excluded by *.log"),
            ("baz.log", "does not match include, excluded"),
        ],
    );

    // Create archive with --include='*foo*' --exclude='*.log'
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
            "--include=*foo*",
            "--exclude=*.log",
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
        ])
        .assert()
        .success();

    // Verify: only foo.txt should be included
    // - foo.txt: matches *foo*, not *.log → INCLUDED
    // - bar.txt: doesn't match *foo* → EXCLUDED
    // - foobar.log: matches *foo*, but also *.log → EXCLUDED (exclude wins)
    // - baz.log: doesn't match *foo*, matches *.log → EXCLUDED
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(
        entry_names.contains("foo.txt"),
        "foo.txt should be included"
    );
    assert!(
        !entry_names.contains("bar.txt"),
        "bar.txt should be excluded (doesn't match include)"
    );
    assert!(
        !entry_names.contains("foobar.log"),
        "foobar.log should be excluded (exclude takes precedence)"
    );
    assert!(
        !entry_names.contains("baz.log"),
        "baz.log should be excluded"
    );
    assert_eq!(entry_names.len(), 1, "Should contain exactly 1 entry");
}

/// Precondition: Source archive contains files with different extensions.
/// Action: Create solid archive with `--exclude` pattern and `@archive` inclusion.
/// Expectation: Entries matching exclude pattern are filtered before solid repack.
#[test]
fn stdio_archive_inclusion_solid_with_filter() {
    setup();

    let base = PathBuf::from("stdio_archive_inclusion_solid_with_filter");
    fs::create_dir_all(&base).unwrap();

    // Create source archive with various file types
    let source_archive = base.join("source.pna");
    create_test_archive(
        &source_archive,
        &[
            ("keep.txt", "keep this content"),
            ("exclude.log", "exclude this"),
            ("also_keep.txt", "also keep this"),
        ],
    );

    // Create solid archive with --exclude='*.log'
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
            "--exclude=*.log",
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
        ])
        .assert()
        .success();

    // Verify only .txt files are included in solid archive
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("keep.txt"), "Missing keep.txt");
    assert!(
        entry_names.contains("also_keep.txt"),
        "Missing also_keep.txt"
    );
    assert!(
        !entry_names.contains("exclude.log"),
        "exclude.log should be filtered"
    );
    assert_eq!(entry_names.len(), 2, "Should contain exactly 2 entries");

    // Verify extraction works and data is correct
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
        fs::read_to_string(extract_dir.join("keep.txt")).unwrap(),
        "keep this content"
    );
    assert_eq!(
        fs::read_to_string(extract_dir.join("also_keep.txt")).unwrap(),
        "also keep this"
    );
    assert!(
        !extract_dir.join("exclude.log").exists(),
        "exclude.log should not be extracted"
    );
}
