#![cfg(not(target_family = "wasm"))]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, Compression, Metadata, WriteOptions};
use std::cell::RefCell;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::rc::Rc;

/// A part writer that appends to one shared buffer, so the parts of a split
/// archive land in it back to back, as they do when piped in order.
#[derive(Clone, Default)]
struct SharedBuf(Rc<RefCell<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Builds a split archive holding `content` under `name`, with the entry data
/// spanning several parts, and returns every part concatenated in order.
fn split_archive_stream(name: &str, content: &[u8]) -> Vec<u8> {
    let buf = SharedBuf::default();
    let mut archive = Archive::write_split_header(300, {
        let buf = buf.clone();
        move |_| Ok(buf.clone())
    })
    .unwrap();
    let options = WriteOptions::builder().compression(Compression::NO).build();
    archive
        .write_file(name.into(), Metadata::new(), options, |w| {
            w.write_all(content)
        })
        .unwrap();
    let parts = archive.finalize().unwrap().parts();
    assert!(
        parts > 1,
        "expected the archive to be split into several parts"
    );
    buf.0.borrow().clone()
}

/// Precondition: Every part of a split archive is piped through stdin, one
///   after another.
/// Action: Run bsdtar-compat list reading from stdin.
/// Expectation: The entry spanning the part boundary is listed, so the reader
///   walked past the first part instead of stopping at it.
#[test]
fn bsdtar_list_reads_split_archive_from_stdin() {
    setup();
    let stream = split_archive_stream("split.txt", &[b'x'; 4096]);

    cargo_bin_cmd!("pna")
        .write_stdin(stream)
        .args(["compat", "bsdtar", "--unstable", "-tf", "-"])
        .assert()
        .success()
        .stdout(predicates::str::contains("split.txt"));
}

/// Precondition: Every part of a split archive is piped through stdin, one
///   after another.
/// Action: Run bsdtar-compat extract reading from stdin.
/// Expectation: The entry spanning the part boundary is restored whole.
#[test]
fn bsdtar_extract_reads_split_archive_from_stdin() {
    setup();
    let stream = split_archive_stream("split.txt", &[b'x'; 4096]);
    let base = PathBuf::from("bsdtar_extract_split_archive_from_stdin");
    fs::create_dir_all(&base).unwrap();

    cargo_bin_cmd!("pna")
        .write_stdin(stream)
        .args([
            "compat",
            "bsdtar",
            "--unstable",
            "--no-xattrs",
            "--cd",
            base.to_str().unwrap(),
            "-xf",
            "-",
        ])
        .assert()
        .success();

    assert_eq!(fs::read(base.join("split.txt")).unwrap(), vec![b'x'; 4096]);
}
