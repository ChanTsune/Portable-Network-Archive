use std::{
    fs, io,
    path::{Path, PathBuf},
};

const MAX_RETRIES: u32 = 3;

/// Atomic file writer using temp file + rename pattern.
///
/// Creates a temporary file in the same directory as the target path,
/// allowing data to be written safely. On [`persist()`](Self::persist), the temp file
/// is atomically renamed to the final path.
///
/// If the `SafeWriter` is dropped without calling `persist()`, the temp file
/// is automatically cleaned up.
pub(crate) struct SafeWriter {
    temp_path: Option<PathBuf>,
    final_path: PathBuf,
    file: fs::File,
}

impl SafeWriter {
    /// Creates a new temp file with pattern `.pna.{random}` in the same directory as `final_path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent directory doesn't exist or temp file creation fails.
    pub(crate) fn new(final_path: impl AsRef<Path>) -> io::Result<Self> {
        let final_path = final_path.as_ref().to_path_buf();
        let parent = final_path.parent().unwrap_or(Path::new("."));

        // Retry on collision (astronomically rare)
        for _ in 0..MAX_RETRIES {
            let random = rand::random::<u64>();
            let temp_name = format!(".pna.{:016x}", random);
            let temp_path = parent.join(temp_name);

            match fs::File::create_new(&temp_path) {
                Ok(file) => {
                    #[cfg(unix)]
                    {
                        // Restrict temp file to owner-only access to prevent other users
                        // from reading sensitive data before final permissions are applied
                        use std::os::unix::fs::PermissionsExt;
                        file.set_permissions(fs::Permissions::from_mode(0o600))?;
                    }
                    return Ok(Self {
                        temp_path: Some(temp_path),
                        final_path,
                        file,
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("failed to create unique temp file after {MAX_RETRIES} attempts"),
        ))
    }

    /// Returns a mutable reference to the underlying file for writing.
    #[inline]
    pub(crate) fn as_file_mut(&mut self) -> &mut fs::File {
        &mut self.file
    }

    /// Atomically renames the temp file to the final path.
    ///
    /// On failure, the temp file is cleaned up automatically via `Drop`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Syncing file data to disk fails (`sync_all`)
    /// - A non-empty directory exists at the destination path
    /// - The destination path cannot be accessed (permission denied, I/O error)
    /// - The rename operation fails
    pub(crate) fn persist(mut self) -> io::Result<()> {
        self.file.sync_all()?;
        self.prepare_destination()?;

        let temp_path = self
            .temp_path
            .take()
            .expect("persist called on already-persisted SafeWriter");
        fs::rename(&temp_path, &self.final_path)
    }

    /// Prepares the destination path for the rename operation.
    fn prepare_destination(&self) -> io::Result<()> {
        #[cfg(windows)]
        use std::os::windows::fs::FileTypeExt;
        match fs::symlink_metadata(&self.final_path) {
            // Empty directory blocking file extraction is removed (non-empty will fail)
            Ok(meta) if meta.file_type().is_dir() => fs::remove_dir(&self.final_path),
            // Windows rename() fails if destination exists; remove it first
            #[cfg(windows)]
            Ok(meta) if meta.file_type().is_symlink_dir() => fs::remove_dir(&self.final_path),
            // Windows rename() fails if destination exists; remove it first
            #[cfg(windows)]
            Ok(_) => fs::remove_file(&self.final_path),
            #[cfg(not(windows))]
            Ok(_) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl io::Write for SafeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Drop for SafeWriter {
    fn drop(&mut self) {
        let Some(ref path) = self.temp_path else {
            return;
        };
        if let Err(e) = fs::remove_file(path)
            && e.kind() != io::ErrorKind::NotFound
        {
            log::warn!("Failed to clean up temp file '{}': {}", path.display(), e);
        }
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join("pna_safe_writer_test");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn safe_writer_creates_temp_in_same_directory() {
        let dir = test_dir();
        let target = dir.join("target.txt");
        let writer = SafeWriter::new(&target).unwrap();

        let temp_path = writer.temp_path.as_ref().unwrap();
        // Temp file should be in same directory
        assert_eq!(temp_path.parent(), target.parent());
        // Temp file should exist
        assert!(temp_path.exists());
        // Temp file name should start with .pna.
        assert!(
            temp_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(".pna.")
        );

        // Cleanup
        drop(writer);
        let _ = fs::remove_file(&target);
    }

    #[test]
    fn safe_writer_persist_renames_atomically() {
        let dir = test_dir();
        let target = dir.join("persist_test.txt");
        let _ = fs::remove_file(&target);

        let mut writer = SafeWriter::new(&target).unwrap();
        let temp_path = writer.temp_path.as_ref().unwrap().clone();

        write!(writer.as_file_mut(), "test content").unwrap();
        writer.persist().unwrap();

        // Final file should exist with content
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "test content");

        // Temp file should be gone
        assert!(!temp_path.exists());

        // Cleanup
        let _ = fs::remove_file(&target);
    }

    #[test]
    fn safe_writer_cleanup_on_drop() {
        let dir = test_dir();
        let target = dir.join("drop_test.txt");
        let temp_path;

        {
            let writer = SafeWriter::new(&target).unwrap();
            temp_path = writer.temp_path.as_ref().unwrap().clone();
            assert!(temp_path.exists());
            // Drop without persist
        }

        // Temp file should be cleaned up
        assert!(!temp_path.exists());
        // Final file should not exist
        assert!(!target.exists());
    }

    #[test]
    fn safe_writer_replaces_existing_file() {
        let dir = test_dir();
        let target = dir.join("replace_test.txt");

        // Create existing file
        fs::write(&target, "old content").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "old content");

        // Create SafeWriter and persist new content
        let mut writer = SafeWriter::new(&target).unwrap();
        write!(writer.as_file_mut(), "new content").unwrap();
        writer.persist().unwrap();

        // File should have new content
        assert_eq!(fs::read_to_string(&target).unwrap(), "new content");

        // Cleanup
        let _ = fs::remove_file(&target);
    }

    #[test]
    fn safe_writer_replaces_empty_directory() {
        let dir = test_dir();
        let target = dir.join("replace_dir_test");

        // Create an empty directory at target path
        let _ = fs::remove_dir_all(&target);
        fs::create_dir(&target).unwrap();
        assert!(target.is_dir());

        // Create SafeWriter and persist - should replace directory with file
        let mut writer = SafeWriter::new(&target).unwrap();
        write!(writer.as_file_mut(), "file content").unwrap();
        writer.persist().unwrap();

        // Target should now be a file, not a directory
        assert!(target.is_file());
        assert_eq!(fs::read_to_string(&target).unwrap(), "file content");

        // Cleanup
        let _ = fs::remove_file(&target);
    }

    #[test]
    fn safe_writer_fails_on_non_empty_directory() {
        let dir = test_dir();
        let target = dir.join("non_empty_dir_test");

        // Create a non-empty directory at target path
        let _ = fs::remove_dir_all(&target);
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inside.txt"), "content").unwrap();
        assert!(target.is_dir());

        // Create SafeWriter and try to persist - should fail
        let mut writer = SafeWriter::new(&target).unwrap();
        write!(writer.as_file_mut(), "file content").unwrap();
        let result = writer.persist();

        // Should fail because directory is not empty
        assert!(result.is_err());

        // Directory should still exist
        assert!(target.is_dir());

        // Cleanup
        let _ = fs::remove_dir_all(&target);
    }
}
