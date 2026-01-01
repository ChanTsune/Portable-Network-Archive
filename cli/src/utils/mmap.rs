use std::{fs, io, ops::Deref};

pub(crate) struct Mmap {
    _file: fs::File,
    inner: memmap2::Mmap,
}

impl Mmap {
    #[inline]
    pub(crate) fn map_with_size(file: fs::File, len: usize) -> io::Result<Self> {
        // SAFETY:
        // - The file handle is kept alive for the lifetime of the map via `_file` field
        // - `len` is obtained from file metadata, so it corresponds to a valid region
        // - The caller must ensure the file is not modified while the map is active;
        //   this is used for reading source files during archive creation where
        //   concurrent modification would cause data corruption regardless of mmap
        let inner = unsafe { memmap2::MmapOptions::new().len(len).map(&file)? };
        Ok(Mmap { _file: file, inner })
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
        // SAFETY:
        // - The file handle is kept alive for the lifetime of the map via `_file` field
        // - The caller must ensure the file is not modified while the map is active
        let inner = unsafe { memmap2::Mmap::map(&file) }?;
        Ok(Mmap { _file: file, inner })
    }
}
