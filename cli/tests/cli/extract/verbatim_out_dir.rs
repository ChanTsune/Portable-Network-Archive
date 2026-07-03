use crate::utils::setup;
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: an archive containing a regular file entry inside a
/// directory, and an existing output directory addressed by its
/// canonicalized (verbatim `\\?\`-prefixed) path.
/// Action: extract the archive with `--out-dir` set to the verbatim path.
/// Expectation: extraction succeeds and the file is restored with its
/// contents under the output directory.
#[test]
fn extract_with_verbatim_out_dir() {
    setup();
    let base = "extract_with_verbatim_out_dir";
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(format!("{base}/in/dir")).unwrap();
    fs::write(format!("{base}/in/dir/file.txt"), b"payload").unwrap();
    let archive = format!("{base}/{base}.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &archive,
        "--overwrite",
        &format!("{base}/in/"),
    ])
    .unwrap()
    .execute()
    .unwrap();

    fs::create_dir_all(format!("{base}/out")).unwrap();
    let out_dir = fs::canonicalize(format!("{base}/out")).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive,
        "--out-dir",
        out_dir.to_str().unwrap(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let restored = out_dir.join(base).join("in/dir/file.txt");
    assert_eq!(fs::read(&restored).unwrap(), b"payload");
}
