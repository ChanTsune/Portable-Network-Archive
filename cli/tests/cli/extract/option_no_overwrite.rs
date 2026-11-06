use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::{fs, path::Path};

/// Precondition: An archive and an existing file have different contents.
/// Action: Extract with the default overwrite policy or explicit --no-overwrite.
/// Expectation: Both invocations reject replacement and preserve the existing file.
#[test]
fn extract_no_overwrite_preserves_existing_file() {
    setup();
    let base = Path::new("extract_no_overwrite_preserves_existing_file");
    fs::create_dir_all(base).unwrap();
    let archive_path = base.join("archive.pna");
    archive::create_test_archive(&archive_path, &[("file.txt", "from archive")]);

    for (output, flags) in [
        ("default", &[][..]),
        ("explicit", &["--no-overwrite"][..]),
    ] {
        let out_dir = base.join(output);
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(out_dir.join("file.txt"), b"existing").unwrap();

        cargo_bin_cmd!("pna")
            .args([
                "--quiet",
                "extract",
                "-f",
                archive_path.to_str().unwrap(),
                "--out-dir",
                out_dir.to_str().unwrap(),
            ])
            .args(flags)
            .assert()
            .failure();

        assert_eq!(
            fs::read(out_dir.join("file.txt")).unwrap(),
            b"existing",
            "{output}"
        );
    }
}
