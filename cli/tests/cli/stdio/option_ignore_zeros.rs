use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, ReadOptions, WriteOptions};
use predicates::prelude::*;
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

const STDIO_DEPRECATION_WARNING: &str = "experimental stdio` was stabilized as";

fn build_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = Archive::write_header(Vec::new()).unwrap();
    for (name, content) in entries {
        let mut builder = EntryBuilder::new_file((*name).into(), WriteOptions::store()).unwrap();
        builder.write_all(content).unwrap();
        archive.add_entry(builder.build().unwrap()).unwrap();
    }
    archive.finalize().unwrap()
}

fn build_concatenated_archives() -> Vec<u8> {
    let mut archives = build_archive(&[("a.txt", b"first" as &[u8])]);
    archives.extend(build_archive(&[("b.txt", b"second" as &[u8])]));
    archives
}

fn build_concatenated_then_split_archive(base: &Path) -> PathBuf {
    let split_dir = base.join("split");
    let source_archive = base.join("source.pna");
    let part1 = split_dir.join("source.part1.pna");

    fs::create_dir_all(&split_dir).unwrap();
    fs::write(
        &source_archive,
        build_archive(&[("split.txt", vec![b'x'; 4096].as_slice())]),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "split",
        source_archive.to_str().unwrap(),
        "--overwrite",
        "--max-size",
        "200",
        "--out-dir",
        split_dir.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stderr("");

    assert!(split_dir.join("source.part2.pna").exists());

    let original_part1 = fs::read(&part1).unwrap();
    let mut mixed_part1 = build_archive(&[("a.txt", b"first" as &[u8])]);
    mixed_part1.extend(original_part1);
    fs::write(&part1, mixed_part1).unwrap();

    part1
}

fn read_archive_entries(path: impl AsRef<Path>) -> Vec<(String, String)> {
    let mut archive = Archive::read_header(fs::File::open(path).unwrap()).unwrap();
    archive
        .entries()
        .extract_solid_entries(&mut ReadOptions::builder().build())
        .map(|entry| {
            let entry = entry.unwrap();
            let mut reader = entry.reader(&mut ReadOptions::builder().build()).unwrap();
            let mut content = String::new();
            reader.read_to_string(&mut content).unwrap();
            (entry.name().to_string(), content)
        })
        .collect()
}

fn read_all_archive_entries_from_bytes(bytes: &[u8]) -> Vec<(String, String)> {
    let mut cursor = Cursor::new(bytes);
    let mut entries = Vec::new();

    loop {
        let mut archive = match Archive::read_header(&mut cursor) {
            Ok(archive) => archive,
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => panic!("unexpected archive read error: {err}"),
        };
        entries.extend(
            archive
                .entries()
                .extract_solid_entries(&mut ReadOptions::builder().build())
                .map(|entry| {
                    let entry = entry.unwrap();
                    let mut reader = entry.reader(&mut ReadOptions::builder().build()).unwrap();
                    let mut content = String::new();
                    reader.read_to_string(&mut content).unwrap();
                    (entry.name().to_string(), content)
                }),
        );
        let _ = archive.into_inner();
    }

    entries
}

fn read_all_archive_entries(path: impl AsRef<Path>) -> Vec<(String, String)> {
    let bytes = fs::read(path).unwrap();
    read_all_archive_entries_from_bytes(&bytes)
}

#[test]
fn stdio_list_ignore_zeros_controls_concatenated_archive_handling() {
    setup();
    let archive_data = build_concatenated_archives();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data.clone())
        .args(["experimental", "stdio", "--list"])
        .assert()
        .success()
        .stdout("a.txt\n")
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--unstable",
            "--list",
            "--ignore-zeros",
        ])
        .assert()
        .success()
        .stdout("a.txt\nb.txt\n")
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));
}

#[test]
fn stdio_list_ignore_zeros_with_fast_read_continues_into_next_archive() {
    setup();
    let archive_data = build_concatenated_archives();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--unstable",
            "--list",
            "--ignore-zeros",
            "--fast-read",
            "b.txt",
        ])
        .assert()
        .success()
        .stdout("b.txt\n")
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));
}

#[test]
fn stdio_extract_ignore_zeros_controls_concatenated_archive_handling() {
    setup();
    let archive_data = build_concatenated_archives();
    let out_without = PathBuf::from("stdio_extract_ignore_zeros_without_flag/out");
    let out_with = PathBuf::from("stdio_extract_ignore_zeros_with_flag/out");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data.clone())
        .args([
            "experimental",
            "stdio",
            "--extract",
            "--out-dir",
            out_without.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        "first",
        fs::read_to_string(out_without.join("a.txt")).unwrap()
    );
    assert!(!out_without.join("b.txt").exists());

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--unstable",
            "--extract",
            "--ignore-zeros",
            "--out-dir",
            out_with.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!("first", fs::read_to_string(out_with.join("a.txt")).unwrap());
    assert_eq!(
        "second",
        fs::read_to_string(out_with.join("b.txt")).unwrap()
    );
}

#[test]
fn stdio_extract_ignore_zeros_with_fast_read_continues_into_next_archive() {
    setup();
    let archive_data = build_concatenated_archives();
    let out_dir = PathBuf::from("stdio_extract_ignore_zeros_fast_read/out");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(archive_data)
        .args([
            "experimental",
            "stdio",
            "--unstable",
            "--extract",
            "--ignore-zeros",
            "--fast-read",
            "--out-dir",
            out_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert!(!out_dir.join("a.txt").exists());
    assert_eq!("second", fs::read_to_string(out_dir.join("b.txt")).unwrap());
}

#[test]
fn stdio_update_ignore_zeros_controls_concatenated_archive_handling() {
    setup();
    let base = PathBuf::from("stdio_update_ignore_zeros");
    let in_dir = base.join("in");
    let archive_without = base.join("without_ignore.pna");
    let archive_with = base.join("with_ignore.pna");

    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("c.txt"), "third").unwrap();
    fs::write(&archive_without, build_concatenated_archives()).unwrap();
    fs::write(&archive_with, build_concatenated_archives()).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--update",
        "--file",
        archive_without.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--unstable",
        "--update",
        "--ignore-zeros",
        "--file",
        archive_with.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}

#[test]
fn stdio_create_ignore_zeros_controls_archive_inclusion_handling() {
    setup();
    let base = PathBuf::from("stdio_create_ignore_zeros");
    let archive_source = base.join("ab-cat.pna");
    let archive_without = base.join("without_ignore.pna");
    let archive_with = base.join("with_ignore.pna");

    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("c.txt"), "third").unwrap();
    fs::write(&archive_source, build_concatenated_archives()).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--create",
        "--file",
        archive_without.to_str().unwrap(),
        "--cd",
        base.to_str().unwrap(),
        "@ab-cat.pna",
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--unstable",
        "--create",
        "--ignore-zeros",
        "--file",
        archive_with.to_str().unwrap(),
        "--cd",
        base.to_str().unwrap(),
        "@ab-cat.pna",
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}

#[test]
fn stdio_create_ignore_zeros_controls_stdin_archive_inclusion_handling() {
    setup();
    let base = PathBuf::from("stdio_create_ignore_zeros_stdin_inclusion");
    let archive_without = base.join("without_ignore.pna");
    let archive_with = base.join("with_ignore.pna");

    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("c.txt"), "third").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(build_concatenated_archives())
        .args([
            "experimental",
            "stdio",
            "--no-xattrs",
            "--create",
            "--file",
            archive_without.to_str().unwrap(),
            "--cd",
            base.to_str().unwrap(),
            "@-",
            "c.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(build_concatenated_archives())
        .args([
            "experimental",
            "stdio",
            "--no-xattrs",
            "--unstable",
            "--create",
            "--ignore-zeros",
            "--file",
            archive_with.to_str().unwrap(),
            "--cd",
            base.to_str().unwrap(),
            "@-",
            "c.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}

#[test]
fn stdio_append_ignore_zeros_controls_existing_concatenated_archive_handling() {
    setup();
    let base = PathBuf::from("stdio_append_ignore_zeros_existing");
    let in_dir = base.join("in");
    let archive_without = base.join("without_ignore.pna");
    let archive_with = base.join("with_ignore.pna");

    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("c.txt"), "third").unwrap();
    fs::write(&archive_without, build_concatenated_archives()).unwrap();
    fs::write(&archive_with, build_concatenated_archives()).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--append",
        "--file",
        archive_without.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_all_archive_entries(&archive_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--unstable",
        "--append",
        "--ignore-zeros",
        "--file",
        archive_with.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_all_archive_entries(&archive_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}

#[test]
fn stdio_update_ignore_zeros_handles_concatenated_archive_before_split_archive() {
    setup();
    let base = PathBuf::from("stdio_update_ignore_zeros_mixed_split");
    let without_base = base.join("without");
    let with_base = base.join("with");
    let archive_without = build_concatenated_then_split_archive(&without_base);
    let archive_with = build_concatenated_then_split_archive(&with_base);
    let in_dir = base.join("in");
    let consolidated_without = without_base.join("split/source.pna");
    let consolidated_with = with_base.join("split/source.pna");

    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("c.txt"), "third").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--update",
        "--file",
        archive_without.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&consolidated_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--unstable",
        "--update",
        "--ignore-zeros",
        "--file",
        archive_with.to_str().unwrap(),
        "--cd",
        in_dir.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&consolidated_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("split.txt".to_string(), "x".repeat(4096)),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}

#[test]
fn stdio_append_ignore_zeros_controls_stdin_base_archive_handling() {
    setup();
    let base = PathBuf::from("stdio_append_ignore_zeros_stdin");
    let in_dir = base.join("in");

    fs::create_dir_all(&in_dir).unwrap();
    fs::write(in_dir.join("c.txt"), "third").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    let output_without = cmd
        .write_stdin(build_concatenated_archives())
        .args([
            "experimental",
            "stdio",
            "--no-xattrs",
            "--append",
            "--cd",
            in_dir.to_str().unwrap(),
            "c.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING))
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        read_all_archive_entries_from_bytes(&output_without),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    let output_with = cmd
        .write_stdin(build_concatenated_archives())
        .args([
            "experimental",
            "stdio",
            "--no-xattrs",
            "--unstable",
            "--append",
            "--ignore-zeros",
            "--cd",
            in_dir.to_str().unwrap(),
            "c.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING))
        .get_output()
        .stdout
        .clone();

    assert_eq!(
        read_all_archive_entries_from_bytes(&output_with),
        vec![
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
            ("c.txt".to_string(), "third".to_string()),
        ]
    );
}

#[test]
fn stdio_append_ignore_zeros_handles_concatenated_archive_before_split_archive() {
    setup();
    let base = PathBuf::from("stdio_append_ignore_zeros_mixed_split");
    let archive = build_concatenated_then_split_archive(&base);

    fs::write(base.join("c.txt"), "third").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--no-xattrs",
        "--unstable",
        "--append",
        "--ignore-zeros",
        "--file",
        archive.to_str().unwrap(),
        "--cd",
        base.to_str().unwrap(),
        "c.txt",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--unstable",
        "--list",
        "--ignore-zeros",
        "--file",
        archive.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout("a.txt\nsplit.txt\nc.txt\n")
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));
}

#[test]
fn stdio_append_ignore_zeros_controls_archive_inclusion_handling() {
    setup();
    let base = PathBuf::from("stdio_append_ignore_zeros_inclusion");
    let archive_source = base.join("ab-cat.pna");
    let archive_without = base.join("without_ignore.pna");
    let archive_with = base.join("with_ignore.pna");

    fs::create_dir_all(&base).unwrap();
    fs::write(&archive_source, build_concatenated_archives()).unwrap();
    fs::write(
        &archive_without,
        build_archive(&[("x.txt", b"base" as &[u8])]),
    )
    .unwrap();
    fs::write(&archive_with, build_archive(&[("x.txt", b"base" as &[u8])])).unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--append",
        "--file",
        archive_without.to_str().unwrap(),
        "--cd",
        base.to_str().unwrap(),
        "@ab-cat.pna",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_without),
        vec![
            ("x.txt".to_string(), "base".to_string()),
            ("a.txt".to_string(), "first".to_string()),
        ]
    );

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "experimental",
        "stdio",
        "--unstable",
        "--append",
        "--ignore-zeros",
        "--file",
        archive_with.to_str().unwrap(),
        "--cd",
        base.to_str().unwrap(),
        "@ab-cat.pna",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains(STDIO_DEPRECATION_WARNING));

    assert_eq!(
        read_archive_entries(&archive_with),
        vec![
            ("x.txt".to_string(), "base".to_string()),
            ("a.txt".to_string(), "first".to_string()),
            ("b.txt".to_string(), "second".to_string()),
        ]
    );
}
