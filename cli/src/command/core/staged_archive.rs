use super::SafeWriter;
use crate::utils::GlobPatterns;
use std::{fs, io, path::PathBuf};

/// An archive rewrite held in a temp file until the checks that must precede it have passed.
///
/// The staged file takes the place of the original only through [`commit()`](Self::commit),
/// so a run that fails its checks leaves the original as it was.
pub(crate) struct StagedArchive {
    writer: SafeWriter,
}

impl StagedArchive {
    /// Stages a rewrite of `output`, creating the directory it lives in if it is missing.
    #[inline]
    pub(crate) fn new(output: PathBuf) -> io::Result<Self> {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self {
            writer: SafeWriter::new(output)?,
        })
    }

    /// Returns the file the rewritten archive is written to.
    #[inline]
    pub(crate) fn as_file_mut(&mut self) -> &mut fs::File {
        self.writer.as_file_mut()
    }

    /// Puts the staged archive in place of the original.
    ///
    /// `patterns` holds the patterns the run was asked to match, or is `None` for commands
    /// that take none. A pattern that matched nothing aborts the commit.
    #[inline]
    pub(crate) fn commit(self, patterns: Option<&GlobPatterns>) -> anyhow::Result<()> {
        if let Some(patterns) = patterns {
            patterns.ensure_all_matched()?;
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

        let staged = StagedArchive::new(output).unwrap();

        assert_eq!(entries_beside(&dir, "archive.pna").len(), 1);
        drop(staged);
    }

    #[test]
    fn dropping_without_commit_keeps_the_archive_and_leaves_no_staged_file() {
        let dir = test_dir("drop");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();

        let mut staged = StagedArchive::new(output.clone()).unwrap();
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

        let mut staged = StagedArchive::new(output.clone()).unwrap();
        staged.as_file_mut().write_all(b"rewritten").unwrap();
        staged.commit(None).unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"rewritten");
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }

    #[test]
    fn commit_is_refused_when_a_pattern_matched_nothing() {
        let dir = test_dir("unmatched");
        let output = dir.join("archive.pna");
        fs::write(&output, b"original").unwrap();

        let mut staged = StagedArchive::new(output.clone()).unwrap();
        staged.as_file_mut().write_all(b"rewritten").unwrap();
        let patterns = GlobPatterns::new(["absent"]).unwrap();

        assert!(staged.commit(Some(&patterns)).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"original");
        assert!(entries_beside(&dir, "archive.pna").is_empty());
    }

    #[test]
    fn a_missing_directory_is_created_for_the_archive() {
        let dir = test_dir("missing_parent");
        let output = dir.join("nested").join("archive.pna");

        let mut staged = StagedArchive::new(output.clone()).unwrap();
        staged.as_file_mut().write_all(b"written").unwrap();
        staged.commit(None).unwrap();

        assert_eq!(fs::read(&output).unwrap(), b"written");
    }
}
