use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    use std::process::Command;

    fn can_mount_tmpfs() -> bool {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            return false;
        }
        let test_path = "/tmp/pna_mount_test";
        let _ = fs::remove_dir_all(test_path);
        if fs::create_dir_all(test_path).is_err() {
            return false;
        }
        let result = Command::new("mount")
            .args(["-t", "tmpfs", "tmpfs", test_path])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if result {
            let _ = Command::new("umount").arg(test_path).status();
        }
        let _ = fs::remove_dir_all(test_path);
        result
    }

    /// Precondition: A directory contains a file and a subdirectory mounted as tmpfs with its own file.
    /// Action: Run `pna create` with `--one-file-system`.
    /// Expectation: The archive contains only entries from the original filesystem; the tmpfs file is excluded.
    /// Note: This test requires root privileges to mount tmpfs.
    #[test]
    fn create_with_one_file_system() {
        setup();
        if !can_mount_tmpfs() {
            eprintln!("Skipping test: cannot create tmpfs mount (requires root)");
            return;
        }

        let _ = fs::remove_dir_all("option_one_file_system_mount");
        fs::create_dir_all("option_one_file_system_mount/in/subdir").unwrap();

        // Defer cleanup to ensure it runs even if the test panics.
        scopeguard::defer! {
            let _ = Command::new("umount").arg("option_one_file_system_mount/in/subdir").status();
            let _ = fs::remove_dir_all("option_one_file_system_mount");
        };

        fs::write("option_one_file_system_mount/in/main_file.txt", "main fs").unwrap();

        let mount_status = Command::new("mount")
            .args([
                "-t",
                "tmpfs",
                "tmpfs",
                "option_one_file_system_mount/in/subdir",
            ])
            .status()
            .expect("failed to execute mount");
        if !mount_status.success() {
            eprintln!("Skipping test: failed to mount tmpfs");
            return;
        }

        fs::write(
            "option_one_file_system_mount/in/subdir/tmpfs_file.txt",
            "tmpfs content",
        )
        .unwrap();

        let main_dev = fs::metadata("option_one_file_system_mount/in/main_file.txt")
            .unwrap()
            .dev();
        let tmpfs_dev = fs::metadata("option_one_file_system_mount/in/subdir/tmpfs_file.txt")
            .unwrap()
            .dev();
        assert_ne!(
            main_dev, tmpfs_dev,
            "files should be on different filesystems"
        );

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "create",
            "option_one_file_system_mount/archive_with_flag.pna",
            "--overwrite",
            "--keep-dir",
            "--unstable",
            "--one-file-system",
            "option_one_file_system_mount/in/",
        ])
        .unwrap()
        .execute()
        .unwrap();

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "create",
            "option_one_file_system_mount/archive_without_flag.pna",
            "--overwrite",
            "--keep-dir",
            "option_one_file_system_mount/in/",
        ])
        .unwrap()
        .execute()
        .unwrap();

        let _ = Command::new("umount")
            .arg("option_one_file_system_mount/in/subdir")
            .status();

        // Verify archive with --one-file-system
        let mut seen = HashSet::new();
        archive::for_each_entry(
            "option_one_file_system_mount/archive_with_flag.pna",
            |entry| {
                seen.insert(entry.header().path().to_string());
            },
        )
        .unwrap();

        assert!(
            seen.contains("option_one_file_system_mount/in"),
            "input directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_mount/in/main_file.txt"),
            "main file should be included"
        );
        assert!(
            !seen.contains("option_one_file_system_mount/in/subdir"),
            "subdir directory should NOT be included (on different filesystem)"
        );
        assert!(
            !seen.contains("option_one_file_system_mount/in/subdir/tmpfs_file.txt"),
            "tmpfs file should NOT be included"
        );
        assert_eq!(
            seen.len(),
            2,
            "Expected exactly 2 entries (directory and main file), but found {}: {seen:?}",
            seen.len()
        );

        // Verify archive without --one-file-system
        let mut seen = HashSet::new();
        archive::for_each_entry(
            "option_one_file_system_mount/archive_without_flag.pna",
            |entry| {
                seen.insert(entry.header().path().to_string());
            },
        )
        .unwrap();

        assert!(
            seen.contains("option_one_file_system_mount/in"),
            "input directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_mount/in/main_file.txt"),
            "main file should be included"
        );
        assert!(
            seen.contains("option_one_file_system_mount/in/subdir"),
            "subdir directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_mount/in/subdir/tmpfs_file.txt"),
            "tmpfs file should be included"
        );
        assert_eq!(
            seen.len(),
            4,
            "Expected exactly 4 entries (2 directories and 2 files), but found {}: {seen:?}",
            seen.len()
        );
    }

    /// Precondition: A directory contains files and subdirectories all on the same filesystem.
    /// Action: Run `pna create` with `--one-file-system`.
    /// Expectation: All entries are included in the archive.
    #[test]
    fn create_with_one_file_system_same_fs() {
        setup();

        let _ = fs::remove_dir_all("option_one_file_system_local");
        fs::create_dir_all("option_one_file_system_local/in/subdir").unwrap();

        fs::write("option_one_file_system_local/in/file1.txt", "content1").unwrap();
        fs::write(
            "option_one_file_system_local/in/subdir/file2.txt",
            "content2",
        )
        .unwrap();

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "create",
            "option_one_file_system_local/archive.pna",
            "--overwrite",
            "--keep-dir",
            "--unstable",
            "--one-file-system",
            "option_one_file_system_local/in/",
        ])
        .unwrap()
        .execute()
        .unwrap();

        let mut seen = HashSet::new();
        archive::for_each_entry("option_one_file_system_local/archive.pna", |entry| {
            seen.insert(entry.header().path().to_string());
        })
        .unwrap();

        assert!(
            seen.contains("option_one_file_system_local/in"),
            "input directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_local/in/file1.txt"),
            "file1.txt should be included"
        );
        assert!(
            seen.contains("option_one_file_system_local/in/subdir"),
            "subdir directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_local/in/subdir/file2.txt"),
            "file2.txt should be included"
        );
        assert_eq!(
            seen.len(),
            4,
            "Expected exactly 4 entries (2 directories and 2 files), but found {}: {seen:?}",
            seen.len()
        );
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
mod platform {
    use super::*;

    /// Precondition: A directory contains files and subdirectories all on the same filesystem.
    /// Action: Run `pna create` with `--one-file-system`.
    /// Expectation: All entries are included in the archive.
    #[test]
    fn create_with_one_file_system_same_fs() {
        setup();

        let _ = fs::remove_dir_all("option_one_file_system_local");
        fs::create_dir_all("option_one_file_system_local/in/subdir").unwrap();

        fs::write("option_one_file_system_local/in/file1.txt", "content1").unwrap();
        fs::write(
            "option_one_file_system_local/in/subdir/file2.txt",
            "content2",
        )
        .unwrap();

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "create",
            "option_one_file_system_local/archive.pna",
            "--overwrite",
            "--keep-dir",
            "--unstable",
            "--one-file-system",
            "option_one_file_system_local/in/",
        ])
        .unwrap()
        .execute()
        .unwrap();

        let mut seen = HashSet::new();
        archive::for_each_entry("option_one_file_system_local/archive.pna", |entry| {
            seen.insert(entry.header().path().to_string());
        })
        .unwrap();

        assert!(
            seen.contains("option_one_file_system_local/in"),
            "input directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_local/in/file1.txt"),
            "file1.txt should be included"
        );
        assert!(
            seen.contains("option_one_file_system_local/in/subdir"),
            "subdir directory should be included"
        );
        assert!(
            seen.contains("option_one_file_system_local/in/subdir/file2.txt"),
            "file2.txt should be included"
        );
        assert_eq!(
            seen.len(),
            4,
            "Expected exactly 4 entries (2 directories and 2 files), but found {}: {seen:?}",
            seen.len()
        );
    }
}
