#[cfg(feature = "memmap")]
use std::borrow::Cow;
use std::{fs, io};

use pna::{NormalEntry, ReadEntry};

use super::TransformStrategy;

pub(crate) struct SplitArchiveReader {
    #[cfg(feature = "memmap")]
    mmaps: Vec<crate::utils::mmap::Mmap>,
    #[cfg(not(feature = "memmap"))]
    files: Vec<fs::File>,
}

impl SplitArchiveReader {
    pub(crate) fn new(files: Vec<fs::File>) -> io::Result<Self> {
        #[cfg(feature = "memmap")]
        {
            let mmaps = files
                .into_iter()
                .map(crate::utils::mmap::Mmap::try_from)
                .collect::<io::Result<Vec<_>>>()?;
            Ok(Self { mmaps })
        }
        #[cfg(not(feature = "memmap"))]
        {
            Ok(Self { files })
        }
    }

    #[cfg(not(feature = "memmap"))]
    pub(crate) fn transform_entries<W, F, S>(
        &mut self,
        writer: W,
        password: Option<&[u8]>,
        processor: F,
        strategy: S,
    ) -> anyhow::Result<()>
    where
        W: io::Write,
        F: FnMut(io::Result<NormalEntry>) -> io::Result<Option<NormalEntry>>,
        S: TransformStrategy,
    {
        super::run_transform_entry(
            writer,
            self.files.drain(..),
            || password,
            processor,
            strategy,
        )
    }

    #[cfg(feature = "memmap")]
    pub(crate) fn transform_entries<'s, W, F, S>(
        &'s mut self,
        writer: W,
        password: Option<&[u8]>,
        processor: F,
        strategy: S,
    ) -> anyhow::Result<()>
    where
        W: io::Write,
        F: FnMut(
            io::Result<NormalEntry<Cow<'s, [u8]>>>,
        ) -> io::Result<Option<NormalEntry<Cow<'s, [u8]>>>>,
        S: TransformStrategy,
    {
        super::run_transform_entry(
            writer,
            self.mmaps.iter().map(|m| m.as_ref()),
            || password,
            processor,
            strategy,
        )
    }

    #[cfg(not(feature = "memmap"))]
    pub(crate) fn for_each_entry(
        &mut self,
        password: Option<&[u8]>,
        processor: impl FnMut(io::Result<NormalEntry>) -> io::Result<()>,
    ) -> io::Result<()> {
        super::run_process_archive(self.files.drain(..), || password, processor)
    }

    #[cfg(feature = "memmap")]
    pub(crate) fn for_each_entry<'s>(
        &'s mut self,
        password: Option<&[u8]>,
        mut processor: impl FnMut(io::Result<NormalEntry<Cow<'s, [u8]>>>) -> io::Result<()>,
    ) -> io::Result<()> {
        super::run_read_entries_mem(
            self.mmaps.iter().map(|m| m.as_ref()),
            |entry| match entry? {
                ReadEntry::Solid(s) => s
                    .entries(password)?
                    .try_for_each(|r| processor(r.map(Into::into))),
                ReadEntry::Normal(n) => processor(Ok(n)),
            },
        )
    }

    #[cfg(not(feature = "memmap"))]
    pub(crate) fn for_each_read_entry(
        &mut self,
        processor: impl FnMut(io::Result<ReadEntry>) -> io::Result<()>,
    ) -> io::Result<()> {
        super::run_read_entries(self.files.drain(..), processor)
    }

    #[cfg(feature = "memmap")]
    pub(crate) fn for_each_read_entry<'s>(
        &'s mut self,
        processor: impl FnMut(io::Result<ReadEntry<Cow<'s, [u8]>>>) -> io::Result<()>,
    ) -> io::Result<()> {
        super::run_read_entries_mem(self.mmaps.iter().map(|m| m.as_ref()), processor)
    }
}
