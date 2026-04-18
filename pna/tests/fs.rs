//! Integration tests for pna::fs utilities.
//!
//! The Windows heuristic in `pna::fs::symlink` probes the target to decide
//! between `symlink_file` and `symlink_dir`. These tests pin the six
//! observable outcomes (relative/absolute × dir/file/missing).
//!
//! The heuristic has one further branch — the `link.parent() == None`
//! fallback — that fires only when the link path is a root (`/`, `C:\`) or
//! empty. Creating a symlink at such a path fails at the OS syscall
//! regardless of flavor choice, so the branch is unreachable in realistic
//! use and is deliberately not exercised.
//!
//! Unix symlink creation is a single-line delegation to stdlib and is not
//! retested here.

#[cfg(windows)]
mod windows_heuristic {
    use std::fs;
    use std::os::windows::fs::FileTypeExt;
    use tempfile::TempDir;

    // --- Relative-target tests ---
    // The `original` path is a bare name ("target_dir" etc.). pna::fs::symlink
    // resolves it against the link's parent (the TempDir) and probes
    // link.parent().join(original).is_dir() — the `is_relative` branch of the
    // heuristic.

    /// Precondition: A directory exists at the target name under the link's parent.
    /// Action: Create a symlink with a relative target.
    /// Expectation: Heuristic selects `symlink_dir`.
    #[test]
    fn windows_symlink_probes_relative_dir_and_picks_symlink_dir() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("target_dir")).unwrap();
        let link = tmp.path().join("link");

        pna::fs::symlink("target_dir", &link).unwrap();

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(
            meta.file_type().is_symlink_dir(),
            "expected symlink_dir flavor; got {:?}",
            meta.file_type()
        );
    }

    /// Precondition: A regular file exists at the target name under the link's parent.
    /// Action: Create a symlink with a relative target.
    /// Expectation: Heuristic selects `symlink_file`.
    #[test]
    fn windows_symlink_probes_relative_file_and_picks_symlink_file() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("target_file"), b"").unwrap();
        let link = tmp.path().join("link");

        pna::fs::symlink("target_file", &link).unwrap();

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(
            meta.file_type().is_symlink_file(),
            "expected symlink_file flavor; got {:?}",
            meta.file_type()
        );
    }

    /// Precondition: Nothing exists at the target name under the link's parent.
    /// Action: Create a symlink with a relative target.
    /// Expectation: Heuristic defaults to `symlink_file` (the fallback branch).
    #[test]
    fn windows_symlink_missing_relative_target_defaults_to_symlink_file() {
        let tmp = TempDir::new().unwrap();
        let link = tmp.path().join("link");

        pna::fs::symlink("missing_relative", &link).unwrap();

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(
            meta.file_type().is_symlink_file(),
            "expected symlink_file (default); got {:?}",
            meta.file_type()
        );
    }

    // --- Absolute-target tests ---
    // The `original` path is an absolute PathBuf. pna::fs::symlink bypasses
    // the `link.parent().join(...)` resolution and probes `original.is_dir()`
    // directly — the `else` branch of the heuristic.

    /// Precondition: An absolute path points to an existing directory.
    /// Action: Create a symlink with the absolute target.
    /// Expectation: Heuristic's absolute-path branch probes `original.is_dir()`
    /// and selects `symlink_dir`.
    #[test]
    fn windows_symlink_probes_absolute_dir_and_picks_symlink_dir() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("abs_target_dir");
        fs::create_dir_all(&target).unwrap();
        let link = tmp.path().join("link");

        pna::fs::symlink(&target, &link).unwrap();

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(
            meta.file_type().is_symlink_dir(),
            "expected symlink_dir flavor from absolute path; got {:?}",
            meta.file_type()
        );
    }

    /// Precondition: An absolute path points to an existing regular file.
    /// Action: Create a symlink with the absolute target.
    /// Expectation: Heuristic's absolute-path branch probes `original.is_dir()`,
    /// finds `false`, and selects `symlink_file`.
    #[test]
    fn windows_symlink_probes_absolute_file_and_picks_symlink_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("abs_target_file");
        fs::write(&target, b"").unwrap();
        let link = tmp.path().join("link");

        pna::fs::symlink(&target, &link).unwrap();

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(
            meta.file_type().is_symlink_file(),
            "expected symlink_file flavor from absolute path; got {:?}",
            meta.file_type()
        );
    }

    /// Precondition: An absolute path points to a location that does not exist.
    /// Action: Create a symlink with the absolute target.
    /// Expectation: Heuristic's absolute-path branch probes `original.is_dir()`,
    /// which returns `false` for missing paths, and defaults to `symlink_file`.
    #[test]
    fn windows_symlink_missing_absolute_target_defaults_to_symlink_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("missing_absolute");
        let link = tmp.path().join("link");

        pna::fs::symlink(&target, &link).unwrap();

        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(
            meta.file_type().is_symlink_file(),
            "expected symlink_file (default) from missing absolute path; got {:?}",
            meta.file_type()
        );
    }
}
