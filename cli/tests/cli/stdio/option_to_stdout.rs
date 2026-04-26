#![cfg(not(target_family = "wasm"))]

use crate::utils::setup;
use assert_cmd::cargo::{CommandCargoExt, cargo_bin_cmd};
use pna::{Archive, EntryBuilder, WriteOptions};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

fn build_mixed_kind_archive() -> Vec<u8> {
    let mut archive = Archive::write_header(Vec::new()).unwrap();

    let mut a = EntryBuilder::new_file("a.txt".into(), WriteOptions::store()).unwrap();
    a.write_all(b"alpha").unwrap();
    archive.add_entry(a.build().unwrap()).unwrap();

    let dir = EntryBuilder::new_dir("dir/".into()).build().unwrap();
    archive.add_entry(dir).unwrap();

    let symlink = EntryBuilder::new_symlink("link.txt".into(), "a.txt".into())
        .unwrap()
        .build()
        .unwrap();
    archive.add_entry(symlink).unwrap();

    let hardlink = EntryBuilder::new_hard_link("hard.txt".into(), "a.txt".into())
        .unwrap()
        .build()
        .unwrap();
    archive.add_entry(hardlink).unwrap();

    let mut b = EntryBuilder::new_file("b.txt".into(), WriteOptions::store()).unwrap();
    b.write_all(b"beta").unwrap();
    archive.add_entry(b.build().unwrap()).unwrap();

    archive.finalize().unwrap()
}

fn build_large_archive(content_size: usize) -> Vec<u8> {
    let mut archive = Archive::write_header(Vec::new()).unwrap();

    let mut builder = EntryBuilder::new_file("big.bin".into(), WriteOptions::store()).unwrap();
    let chunk = vec![0u8; 4096];
    let mut remaining = content_size;
    while remaining > 0 {
        let to_write = chunk.len().min(remaining);
        builder.write_all(&chunk[..to_write]).unwrap();
        remaining -= to_write;
    }
    archive.add_entry(builder.build().unwrap()).unwrap();
    archive.finalize().unwrap()
}

/// Precondition: Archive contains a mix of File and non-File entries.
/// Action: Extract to stdout from stdin.
/// Expectation: Only File entry contents appear on stdout.
#[test]
fn stdio_extract_with_to_stdout_skips_non_file_entries() {
    setup();
    let archive_data = build_mixed_kind_archive();

    cargo_bin_cmd!("pna")
        .write_stdin(archive_data)
        .args(["compat", "bsdtar", "--unstable", "-xOf", "-"])
        .assert()
        .success()
        .stdout("alphabeta");
}

/// Precondition: Archive whose extracted content exceeds the OS pipe buffer.
/// Action: Spawn extract-to-stdout; the downstream reader closes after
///   consuming a small prefix, forcing subsequent writes to fail with
///   BrokenPipe.
/// Expectation: The process exits successfully because the broken pipe is
///   treated as a clean termination signal.
#[test]
fn stdio_extract_with_to_stdout_handles_broken_pipe() {
    setup();
    let archive_data = build_large_archive(1 << 20);

    let mut child = Command::cargo_bin("pna")
        .unwrap()
        .args(["compat", "bsdtar", "--unstable", "-xOf", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&archive_data);
    });

    {
        let mut stdout = child.stdout.take().unwrap();
        let mut buf = [0u8; 16];
        let n = stdout.read(&mut buf).expect("failed to read from stdout");
        assert!(
            n > 0,
            "expected to read some data from stdout before closing"
        );
    }

    let status = child.wait().unwrap();
    let _ = writer.join();

    assert!(
        status.success(),
        "pna should exit cleanly on broken pipe, got: {status:?}"
    );
}
