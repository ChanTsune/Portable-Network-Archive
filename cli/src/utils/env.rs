use crate::utils;
use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn temp_dir_or_else<'p>(default: impl Fn() -> &'p Path) -> Cow<'p, Path> {
    if cfg!(target_os = "wasi") {
        default().into()
    } else {
        std::env::temp_dir().into()
    }
}

pub(crate) struct NamedTempFile {
    file_path: PathBuf,
    file: fs::File,
}

impl NamedTempFile {
    #[inline]
    pub(crate) fn new<'p>(fallback_dir: impl Fn() -> &'p Path) -> io::Result<Self> {
        let temp_dir = temp_dir_or_else(fallback_dir);
        fs::create_dir_all(&temp_dir)?;
        let random = rand::random::<u64>();
        let file_path = temp_dir.join(format!("{random}.tmp"));
        let file = fs::File::create(&file_path)?;
        Ok(Self { file, file_path })
    }

    #[inline]
    pub(crate) fn as_file_mut(&mut self) -> &mut fs::File {
        &mut self.file
    }

    #[inline]
    pub(crate) fn persist(self, new_path: impl AsRef<Path>) -> io::Result<()> {
        let Self { file, file_path } = self;
        file.sync_all()?;
        drop(file);

        let new_path_ref = new_path.as_ref();
        if let Some(parent) = new_path_ref.parent() {
            fs::create_dir_all(parent)?;
        }
        utils::fs::mv(file_path, new_path_ref)
    }
}
