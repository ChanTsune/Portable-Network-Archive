#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use crate::utils::{archive, setup};
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use clap::Parser;
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use portable_network_archive::cli;
#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
use std::{fs, path::Path, process::Command as StdCommand, sync::OnceLock};

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
mod platform {
    use super::*;

    /// Checks if nodump is supported in the current working directory.
    /// Must be called after setup() to test in the actual test directory.
    /// The probe runs once per process and the result is cached, so tests
    /// running in parallel cannot race on the shared probe directory.
    fn is_nodump_supported() -> bool {
        static NODUMP_SUPPORTED: OnceLock<bool> = OnceLock::new();
        *NODUMP_SUPPORTED.get_or_init(|| {
            let test_dir = Path::new("pna_nodump_check");
            let _ = fs::remove_dir_all(test_dir);

            let result = (|| -> std::io::Result<()> {
                fs::create_dir_all(test_dir)?;
                let test_file = test_dir.join("test.txt");
                fs::write(&test_file, "test")?;
                try_set_nodump(&test_file).map_err(std::io::Error::other)
            })();

            let _ = fs::remove_dir_all(test_dir);
            if let Err(e) = &result {
                eprintln!("nodump probe failed: {e}");
            }
            result.is_ok()
        })
    }

    #[cfg(target_os = "linux")]
    fn try_set_nodump(path: &Path) -> Result<(), String> {
        match StdCommand::new("chattr").arg("+d").arg(path).output() {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("chattr failed: {stderr}"))
            }
            Err(e) => Err(format!("chattr not available: {e}")),
        }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    fn try_set_nodump(path: &Path) -> Result<(), String> {
        match StdCommand::new("chflags").arg("nodump").arg(path).output() {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("chflags failed: {stderr}"))
            }
            Err(e) => Err(format!("chflags not available: {e}")),
        }
    }

    #[cfg(target_os = "linux")]
    fn set_nodump(path: &Path) {
        let status = StdCommand::new("chattr")
            .arg("+d")
            .arg(path)
            .status()
            .expect("failed to execute chattr");
        assert!(status.success());
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
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
        skip_unless!("nodump", is_nodump_supported());

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
            "-f",
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
        skip_unless!("nodump", is_nodump_supported());

        let _ = fs::remove_dir_all("nodump_append");
        fs::create_dir_all("nodump_append").unwrap();

        let archive_path = "nodump_append/archive.pna";
        let file_path = "nodump_append/file.txt";
        fs::write(file_path, "test").unwrap();
        set_nodump(Path::new(file_path));

        // Create an empty archive first.
        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "create",
            "-f",
            archive_path,
            "--overwrite",
        ])
        .unwrap()
        .execute()
        .unwrap();

        cli::Cli::try_parse_from([
            "pna",
            "--quiet",
            "append",
            "-f",
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
