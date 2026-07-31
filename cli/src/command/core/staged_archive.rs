use crate::utils::{self, GlobPatterns};
use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
};

/// An archive rewrite held in a temp file until the checks that must precede it have passed.
///
/// The staged file takes the place of the original only through [`commit()`](Self::commit),
/// so a run that fails its checks leaves the original as it was.
pub(crate) struct StagedArchive {
    temp: NamedTempFile,
    output: PathBuf,
}

impl StagedArchive {
    /// Stages a rewrite of `output`.
    ///
    /// On platforms without a temp directory the staged file is created next to `output`.
    #[inline]
    pub(crate) fn new(output: PathBuf) -> io::Result<Self> {
        let temp = NamedTempFile::new(|| output.parent().unwrap_or_else(|| ".".as_ref()))?;
        Ok(Self { temp, output })
    }

    /// Returns the file the rewritten archive is written to.
    #[inline]
    pub(crate) fn as_file_mut(&mut self) -> &mut fs::File {
        self.temp.as_file_mut()
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
        self.temp.persist(self.output)?;
        Ok(())
    }
}

fn temp_dir_or_else<'p>(default: impl Fn() -> &'p Path) -> Cow<'p, Path> {
    if cfg!(target_os = "wasi") {
        default().into()
    } else {
        std::env::temp_dir().into()
    }
}

/// Path of a staged file, removed when it goes out of scope.
///
/// Owning the path separately from the open file keeps [`NamedTempFile::persist`] able to
/// close the file before the move while the abandoned path is still cleaned up.
struct TempPath(PathBuf);

impl Drop for TempPath {
    fn drop(&mut self) {
        match fs::remove_file(&self.0) {
            Ok(()) => {}
            // `persist` moved the file away, so nothing is left to remove.
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("failed to remove staged file '{}': {e}", self.0.display()),
        }
    }
}

struct NamedTempFile {
    file_path: TempPath,
    file: fs::File,
}

impl NamedTempFile {
    #[inline]
    fn new<'p>(fallback_dir: impl Fn() -> &'p Path) -> io::Result<Self> {
        let temp_dir = temp_dir_or_else(fallback_dir);
        fs::create_dir_all(&temp_dir)?;
        let random = rand::random::<usize>();
        let file_path = temp_dir.join(format!("{random}.tmp"));
        let file = fs::File::create(&file_path)?;
        Ok(Self {
            file,
            file_path: TempPath(file_path),
        })
    }

    #[inline]
    fn as_file_mut(&mut self) -> &mut fs::File {
        &mut self.file
    }

    #[inline]
    fn persist(self, new_path: impl AsRef<Path>) -> io::Result<()> {
        let Self { file, file_path } = self;
        file.sync_all()?;
        drop(file);

        let new_path_ref = new_path.as_ref();
        if let Some(parent) = new_path_ref.parent() {
            fs::create_dir_all(parent)?;
        }
        utils::fs::mv(&file_path.0, new_path_ref)
    }
}
