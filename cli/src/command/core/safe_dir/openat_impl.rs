use std::io;
use std::path::Path;

pub(crate) struct SafeDir {
    inner: cap_std::fs::Dir,
    #[allow(dead_code)]
    secure_symlinks: bool,
}

impl SafeDir {
    pub(crate) fn open(path: &Path, secure_symlinks: bool) -> io::Result<Self> {
        let inner = cap_std::fs::Dir::open_ambient_dir(path, cap_std::ambient_authority())?;
        Ok(Self {
            inner,
            secure_symlinks,
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            inner: self.inner.try_clone()?,
            secure_symlinks: self.secure_symlinks,
        })
    }
}
