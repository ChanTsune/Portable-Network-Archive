use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

/// Precondition: An archive and an existing file have different contents.
/// Action: Extract with the default overwrite policy or explicit --no-overwrite.
/// Expectation: Both invocations reject replacement and preserve the existing file.
#[test]
fn extract_no_overwrite_preserves_existing_file() {
    setup();
    let archive_path = "extract_no_overwrite_preserves_existing_file/archive.pna";
    archive::create_test_archive(archive_path, &[("file.txt", "from archive")]);

    for (output, flags) in [("default", &[][..]), ("explicit", &["--no-overwrite"][..])] {
        let out_dir = format!("extract_no_overwrite_preserves_existing_file/{output}");
        fs::create_dir_all(&out_dir).unwrap();
        fs::write(format!("{out_dir}/file.txt"), b"existing").unwrap();

        let result = cli::Cli::try_parse_from(
            [
                "pna",
                "--quiet",
                "extract",
                "-f",
                archive_path,
                "--out-dir",
                out_dir.as_str(),
            ]
            .into_iter()
            .chain(flags.iter().copied()),
        )
        .unwrap()
        .execute();

        assert!(result.is_err(), "{output}");
        assert_eq!(
            fs::read(format!("{out_dir}/file.txt")).unwrap(),
            b"existing",
            "{output}"
        );
    }
}
