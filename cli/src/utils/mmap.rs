use std::{fs, io, ops::Deref, path::Path};

pub(crate) struct Mmap {
    _file: fs::File,
    inner: memmap2::Mmap,
}

impl Mmap {
    #[inline]
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> io::Result<Mmap> {
        fn inner(path: &Path) -> io::Result<Mmap> {
            let file = fs::File::open(path)?;
            Mmap::try_from(file)
        }
        inner(path.as_ref())
    }
}

impl AsRef<[u8]> for Mmap {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl Deref for Mmap {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl TryFrom<fs::File> for Mmap {
    type Error = io::Error;
    #[inline]
    fn try_from(file: fs::File) -> Result<Self, Self::Error> {
        let inner = unsafe { memmap2::Mmap::map(&file) }?;
        Ok(Mmap { _file: file, inner })
    }
}
