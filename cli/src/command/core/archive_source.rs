use std::borrow::Cow;
use std::{fs, io};

use pna::{NormalEntry, ReadEntry, ReadOptions};

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
        super::run_transform_entries_readers(
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
        super::run_transform_entries_bytes(
            writer,
            self.mmaps.iter().map(|m| m.as_ref()),
            || password,
            processor,
            strategy,
        )
    }

    #[cfg(not(feature = "memmap"))]
    pub(crate) fn for_each_entry<'s>(
        &'s mut self,
        read_options: &ReadOptions,
        mut processor: impl FnMut(io::Result<NormalEntry<Cow<'s, [u8]>>>) -> io::Result<()>,
    ) -> io::Result<()> {
        super::run_process_archive_readers(
            self.files.drain(..),
            read_options,
            |entry| processor(entry.map(Into::into)),
            false,
        )
    }

    #[cfg(feature = "memmap")]
    pub(crate) fn for_each_entry<'s>(
        &'s mut self,
        read_options: &ReadOptions,
        processor: impl FnMut(io::Result<NormalEntry<Cow<'s, [u8]>>>) -> io::Result<()>,
    ) -> io::Result<()> {
        super::run_process_archive_bytes(
            self.mmaps.iter().map(|m| m.as_ref()),
            read_options,
            processor,
        )
    }

    #[cfg(not(feature = "memmap"))]
    pub(crate) fn for_each_read_entry<'s>(
        &'s mut self,
        mut processor: impl FnMut(io::Result<ReadEntry<Cow<'s, [u8]>>>) -> io::Result<()>,
        allow_concatenated_archives: bool,
    ) -> io::Result<()> {
        super::run_read_entries_readers(
            self.files.drain(..),
            |entry| processor(entry.map(Into::into)),
            allow_concatenated_archives,
        )
    }

    #[cfg(feature = "memmap")]
    pub(crate) fn for_each_read_entry<'s>(
        &'s mut self,
        processor: impl FnMut(io::Result<ReadEntry<Cow<'s, [u8]>>>) -> io::Result<()>,
        allow_concatenated_archives: bool,
    ) -> io::Result<()> {
        super::run_read_entries_bytes(
            self.mmaps.iter().map(|m| m.as_ref()),
            processor,
            allow_concatenated_archives,
        )
    }
}
