use crate::command::core::{SafeWriter, Umask};
use std::{fs, io, path::PathBuf};

/// An archive rewrite held in a temp file until the checks that must precede it have passed.
///
/// The staged file takes the place of the original only through [`commit()`](Self::commit),
/// so a run that fails its checks leaves the original as it was.
pub(crate) struct StagedArchive {
    writer: SafeWriter,
    #[cfg(unix)]
    mode: u32,
}

impl StagedArchive {
    /// Stages a rewrite of `output`, creating the directory it lives in if it is missing.
    ///
    /// On unix, the commit gives the archive the permission bits of the regular file it
    /// replaces, or what `umask` leaves of `0666` when there is no such file.
    ///
    /// When `overwrite` is false, [`commit()`](Self::commit) atomically refuses to
    /// replace an existing destination instead of renaming over it.
    #[inline]
    #[cfg_attr(not(unix), allow(unused_variables))]
    pub(crate) fn new(output: PathBuf, umask: Umask, overwrite: bool) -> io::Result<Self> {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            match fs::symlink_metadata(&output) {
                Ok(meta) if meta.file_type().is_file() => meta.permissions().mode() & 0o777,
                // A directory, symlink or fifo is replaced by a file the commit creates, so
                // there is no mode being kept.
                Ok(_) => umask.apply(0o666).into(),
                Err(e) if e.kind() == io::ErrorKind::NotFound => umask.apply(0o666).into(),
                Err(e) => return Err(e),
            }
        };
        Ok(Self {
            writer: SafeWriter::new(output, overwrite)?,
            #[cfg(unix)]
            mode,
        })
    }

    /// Returns the file the rewritten archive is written to.
    #[inline]
    pub(crate) fn as_file_mut(&mut self) -> &mut fs::File {
        self.writer.as_file_mut()
    }

    /// Puts the staged archive in place of the original.
    #[inline]
    #[cfg_attr(not(unix), allow(unused_mut))]
    pub(crate) fn commit(mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            self.writer
                .as_file_mut()
                .set_permissions(fs::Permissions::from_mode(self.mode))?;
        }
        self.writer.persist()?;
        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
mod tests {
    use super::*;
    use std::{io::Write, path::Path};

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("pna_staged_archive_test")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries_beside(dir: &Path, archive: &str) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name != archive)
            .collect()
    }

    #[test]
    fn staged_file_is_created_beside_the_archive() {
        let dir = test_dir("beside");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();

        let staged = StagedArchive::new(output, Umask::new(0o022), true).unwrap();

        assert_eq!(entries_beside(&dir, "archive.pna").len(), 1);
        drop(staged);
    }

    #[test]
    fn dropping_without_commit_keeps_the_archive_and_leaves_no_staged_file() {
        let dir = test_dir("drop");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"rewritten").unwrap();
        drop(staged);

        assert_eq!(fs::read(&output).unwrap(), b"original");
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }

    #[test]
    fn commit_replaces_the_archive_contents() {
        let dir = test_dir("commit");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"rewritten").unwrap();
        staged.commit().unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"rewritten");
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }

    #[cfg(unix)]
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[cfg(unix)]
    #[test]
    fn commit_keeps_the_mode_of_the_archive_it_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let dir = test_dir("inherit_mode");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();
        fs::set_permissions(&output, fs::Permissions::from_mode(0o666)).unwrap();

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"rewritten").unwrap();
        staged.commit().unwrap();

        assert_eq!(mode_of(&output), 0o666);
    }

    #[cfg(unix)]
    #[test]
    fn commit_drops_the_special_bits_of_the_archive_it_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let dir = test_dir("special_bits");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();
        fs::set_permissions(&output, fs::Permissions::from_mode(0o4755)).unwrap();

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"rewritten").unwrap();
        staged.commit().unwrap();

        assert_eq!(mode_of(&output), 0o755);
    }

    #[cfg(unix)]
    #[test]
    fn commit_gives_a_new_archive_what_the_umask_leaves_of_0666() {
        let dir = test_dir("new_mode");
        let output = dir.join("archive.pna");

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o007), true).unwrap();
        staged.as_file_mut().write_all(b"written").unwrap();
        staged.commit().unwrap();

        assert_eq!(mode_of(&output), 0o660);
    }

    #[cfg(unix)]
    #[test]
    fn commit_does_not_take_the_mode_of_a_directory_it_replaces() {
        use std::os::unix::fs::PermissionsExt;
        let dir = test_dir("replaced_directory");
        let output = dir.join("archive.pna");
        fs::create_dir(&output).unwrap();
        fs::set_permissions(&output, fs::Permissions::from_mode(0o755)).unwrap();

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"written").unwrap();
        staged.commit().unwrap();

        assert!(output.is_file());
        assert_eq!(mode_of(&output), 0o644);
    }

    #[cfg(unix)]
    #[test]
    fn commit_does_not_take_the_mode_through_a_symlink_it_replaces() {
        use std::os::unix::fs::PermissionsExt;
        let dir = test_dir("replaced_symlink");
        let target = dir.join("target.bin");
        let output = dir.join("archive.pna");
        fs::write(&target, b"target").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        std::os::unix::fs::symlink(&target, &output).unwrap();

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"written").unwrap();
        staged.commit().unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"written");
        assert_eq!(mode_of(&output), 0o644);
        assert_eq!(fs::read(&target).unwrap(), b"target");
        assert_eq!(mode_of(&target), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn new_fails_when_the_mode_of_the_output_cannot_be_read() {
        let dir = test_dir("unreadable_mode");
        let output = dir.join("a".repeat(300));

        assert!(StagedArchive::new(output, Umask::new(0o022), true).is_err());
        assert!(fs::read_dir(&dir).unwrap().next().is_none());
    }

    #[test]
    fn a_missing_directory_is_created_for_the_archive() {
        let dir = test_dir("missing_parent");
        let output = dir.join("nested").join("archive.pna");

        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), true).unwrap();
        staged.as_file_mut().write_all(b"written").unwrap();
        staged.commit().unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"written");
    }

    #[test]
    fn noclobber_commit_refuses_an_existing_archive() {
        let dir = test_dir("noclobber");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();
        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), false).unwrap();
        staged.as_file_mut().write_all(b"replacement").unwrap();
        let error = staged.commit().unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&output).unwrap(), b"original");
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }

    #[test]
    fn noclobber_commit_publishes_when_the_archive_is_absent() {
        let dir = test_dir("noclobber_absent");
        let output = dir.join("archive.pna");
        let _ = fs::remove_file(&output);
        let mut staged = StagedArchive::new(output.clone(), Umask::new(0o022), false).unwrap();
        staged.as_file_mut().write_all(b"written").unwrap();
        staged.commit().unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"written");
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }
}
