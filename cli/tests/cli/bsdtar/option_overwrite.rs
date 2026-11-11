use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::{fs, path::Path};

/// Precondition: An archive and existing destination files contain different data.
/// Action: Extract in bsdtar mode with default and explicitly disabled overwriting.
/// Expectation: Default extraction replaces the file; --no-overwrite preserves it.
#[test]
fn bsdtar_extract_no_overwrite_preserves_existing_file() {
    setup();
    let base = Path::new("bsdtar_extract_no_overwrite");
    fs::create_dir_all(base).unwrap();
    let archive_path = base.join("archive.pna");
    archive::create_test_archive(&archive_path, &[("file.txt", "from archive")]);

    let out_default = base.join("default");
    fs::create_dir_all(&out_default).unwrap();
    fs::write(out_default.join("file.txt"), b"existing").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--extract",
            "-f",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_default.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read(out_default.join("file.txt")).unwrap(),
        b"from archive"
    );

    let out_no_overwrite = base.join("no_overwrite");
    fs::create_dir_all(&out_no_overwrite).unwrap();
    fs::write(out_no_overwrite.join("file.txt"), b"existing").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--extract",
            "--no-overwrite",
            "-f",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_no_overwrite.to_str().unwrap(),
        ])
        .assert()
        .failure();

    assert_eq!(
        fs::read(out_no_overwrite.join("file.txt")).unwrap(),
        b"existing"
    );
}
