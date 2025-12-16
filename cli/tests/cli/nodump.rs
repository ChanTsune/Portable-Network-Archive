use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path, process::Command as StdCommand};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "freebsd"))]
mod platform {
    use super::*;

    #[cfg(target_os = "linux")]
    fn set_nodump(path: &Path) {
        let status = StdCommand::new("chattr")
            .arg("+d")
            .arg(path)
            .status()
            .expect("failed to execute chattr");
        assert!(status.success());
    }

    #[cfg(any(target_os = "macos", target_os = "freebsd"))]
    fn set_nodump(path: &Path) {
        let status = StdCommand::new("chflags")
            .arg("nodump")
            .arg(path)
            .status()
            .expect("failed to execute chflags");
        assert!(status.success());
    }

    fn assert_archive_empty(path: &str) {
        let mut has_entries = false;
        archive::for_each_entry(path, |_entry| {
            has_entries = true;
        })
        .unwrap();
        assert!(
            !has_entries,
            "archive '{path}' should not contain any entries"
        );
    }

    #[test]
    fn create_nodump() {
        setup();
        let _ = fs::remove_dir_all("nodump_create");
        fs::create_dir_all("nodump_create").unwrap();

        let archive_path = "nodump_create/archive.pna";
        let file_path = "nodump_create/file.txt";
        fs::write(file_path, "test").unwrap();
        set_nodump(Path::new(file_path));

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "create",
            archive_path,
            "--overwrite",
            "--unstable",
            "--nodump",
            file_path,
        ])
        .unwrap()
        .execute()
        .unwrap();

        assert_archive_empty(archive_path);
    }

    #[test]
    fn append_nodump() {
        setup();
        let _ = fs::remove_dir_all("nodump_append");
        fs::create_dir_all("nodump_append").unwrap();

        let archive_path = "nodump_append/archive.pna";
        let file_path = "nodump_append/file.txt";
        fs::write(file_path, "test").unwrap();
        set_nodump(Path::new(file_path));

        // Create an empty archive first.
        cli::Cli::try_parse_from(["pna", "--quiet", "create", archive_path, "--overwrite"])
            .unwrap()
            .execute()
            .unwrap();

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "append",
            archive_path,
            "--unstable",
            "--nodump",
            file_path,
        ])
        .unwrap()
        .execute()
        .unwrap();

        assert_archive_empty(archive_path);
    }
}
