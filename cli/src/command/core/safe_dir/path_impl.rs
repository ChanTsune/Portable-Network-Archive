use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct SafeDir {
    #[allow(dead_code)]
    base_path: PathBuf,
    #[allow(dead_code)]
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let base_path = path.to_path_buf();
        Ok(Self {
            base_path,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            base_path: self.base_path.clone(),
            secure_symlinks: self.secure_symlinks,
        })
    }
}
