use std::{borrow::Cow, fs, io, path::PathBuf};

use pna::{NormalEntry, ReadEntry, ReadOptions};

use super::{ArchiveSource, TransformStrategy};
use crate::cli::ArchiveFileArgs;

impl ArchiveSource {
    pub(crate) fn require_file(self) -> anyhow::Result<PathBuf> {
        match self {
            Self::File(path) => Ok(path),
            Self::Stdin => anyhow::bail!(
                "archive input from standard input is not supported by this command yet; specify the archive with --file"
            ),
        }
    }

    pub(crate) fn open(self) -> anyhow::Result<OpenArchiveSource> {
        match self {
            Self::File(path) => {
                let source = SplitArchiveReader::new(super::collect_split_archives(path)?)?;
                Ok(source.into())
            }
            Self::Stdin => Ok(OpenArchiveSource(Repr::Stdin(
                io::BufReader::with_capacity(READ_BUFFER_SIZE, io::stdin().lock()),
            ))),
        }
    }
}

impl From<Option<PathBuf>> for ArchiveSource {
    fn from(path: Option<PathBuf>) -> Self {
        match path {
            Some(path) => Self::File(path),
            None => Self::Stdin,
        }
    }
}

impl ArchiveFileArgs {
    pub(crate) fn source(&self) -> ArchiveSource {
        self.file.clone().into()
    }

    pub(crate) fn require_file(&self) -> anyhow::Result<PathBuf> {
        self.source().require_file()
    }
}

pub(crate) struct SplitArchiveReader {
    #[cfg(feature = "memmap")]
    mmaps: Vec<crate::utils::mmap::Mmap>,
    #[cfg(not(feature = "memmap"))]
    files: Vec<fs::File>,
}

const READ_BUFFER_SIZE: usize = 64 * 1024;

/// An archive input opened for reading. The representation is private so every consumer
/// goes through the entry, transform, or [`consume`](Self::consume) methods instead of
/// branching on where the bytes come from.
pub(crate) struct OpenArchiveSource(Repr);

enum Repr {
    File(SplitArchiveReader),
    Stdin(io::BufReader<io::StdinLock<'static>>),
}

impl From<SplitArchiveReader> for OpenArchiveSource {
    fn from(reader: SplitArchiveReader) -> Self {
        Self(Repr::File(reader))
    }
}

/// Continuation run with the opened source. The reader's concrete type is only known once
/// the source is opened, so the read path is expressed as a generic method and monomorphizes
/// per source instead of dispatching on every read.
pub(crate) trait SourceConsumer {
    type Output;

    /// The archive parts as sequential readers.
    fn readers<R: io::Read, I: Iterator<Item = R>>(self, readers: I) -> Self::Output;

    /// The archive parts as memory-mapped bytes. The default streams the mapped bytes
    /// through [`readers`](Self::readers) (`&[u8]` implements `Read`), so only consumers
    /// with a slice-native fast path override it.
    #[cfg(feature = "memmap")]
    fn bytes<'d>(self, parts: impl Iterator<Item = &'d [u8]> + Send) -> Self::Output
    where
        Self: Sized,
    {
        self.readers(parts)
    }
}

/// Receives each entry of an archive. Entry data is borrowed from a memory map or owned by
/// a read buffer depending on the source; both arrive as `Cow`, so one visitor serves every
/// source.
pub(crate) trait EntryVisitor {
    fn visit(&mut self, entry: NormalEntry<Cow<'_, [u8]>>) -> io::Result<()>;
}

impl<V: EntryVisitor + ?Sized> EntryVisitor for &mut V {
    fn visit(&mut self, entry: NormalEntry<Cow<'_, [u8]>>) -> io::Result<()> {
        (**self).visit(entry)
    }
}

/// Like [`EntryVisitor`], but sees solid blocks as a whole instead of their members.
pub(crate) trait ReadEntryVisitor {
    fn visit(&mut self, entry: ReadEntry<Cow<'_, [u8]>>) -> io::Result<()>;
}

impl<V: ReadEntryVisitor + ?Sized> ReadEntryVisitor for &mut V {
    fn visit(&mut self, entry: ReadEntry<Cow<'_, [u8]>>) -> io::Result<()> {
        (**self).visit(entry)
    }
}

impl OpenArchiveSource {
    pub(crate) fn for_each_entry(
        self,
        read_options: &ReadOptions,
        mut visitor: impl EntryVisitor,
    ) -> io::Result<()> {
        match self.0 {
            Repr::File(mut source) => {
                source.for_each_entry(read_options, |entry| visitor.visit(entry?))
            }
            Repr::Stdin(stdin) => super::run_process_archive_readers(
                [stdin],
                read_options,
                |entry| visitor.visit(entry?.into()),
                false,
            ),
        }
    }

    pub(crate) fn for_each_read_entry(
        self,
        mut visitor: impl ReadEntryVisitor,
        allow_concatenated_archives: bool,
    ) -> io::Result<()> {
        match self.0 {
            Repr::File(mut source) => source
                .for_each_read_entry(|entry| visitor.visit(entry?), allow_concatenated_archives),
            Repr::Stdin(stdin) => super::run_read_entries_readers(
                [stdin],
                |entry| visitor.visit(entry?.into()),
                allow_concatenated_archives,
            ),
        }
    }

    /// Rewrites every entry through `transform` into `writer`; `strategy` decides whether
    /// solid blocks are kept or expanded on the way.
    pub(crate) fn transform_entries<W, S>(
        self,
        writer: W,
        password: Option<&[u8]>,
        transform: &mut impl super::rewrite::EntryTransform,
        strategy: S,
    ) -> anyhow::Result<()>
    where
        W: io::Write,
        S: TransformStrategy,
    {
        match self.0 {
            Repr::File(mut source) => source.transform_entries(
                writer,
                password,
                |entry| transform.transform(entry?),
                strategy,
            ),
            Repr::Stdin(stdin) => super::run_transform_entries_readers(
                writer,
                [stdin],
                || password,
                |entry| transform.transform(entry?),
                strategy,
            ),
        }
    }

    /// Hands the archive parts to `consumer`, as memory-mapped bytes when this build maps
    /// file sources and as buffered readers otherwise.
    pub(crate) fn consume<C: SourceConsumer>(self, consumer: C) -> C::Output {
        match self.0 {
            #[cfg(feature = "memmap")]
            Repr::File(source) => consumer.bytes(source.bytes()),
            #[cfg(not(feature = "memmap"))]
            Repr::File(source) => consumer.readers(
                source
                    .into_readers()
                    .map(|file| io::BufReader::with_capacity(READ_BUFFER_SIZE, file)),
            ),
            Repr::Stdin(stdin) => consumer.readers(std::iter::once(stdin)),
        }
    }
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
    pub(crate) fn into_readers(self) -> impl Iterator<Item = fs::File> {
        self.files.into_iter()
    }

    #[cfg(feature = "memmap")]
    pub(crate) fn bytes(&self) -> impl Iterator<Item = &[u8]> {
        self.mmaps.iter().map(|mmap| mmap.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn archive_argument_resolves_omission_but_keeps_dash_literal() {
        let omitted = ArchiveFileArgs { file: None };
        assert_eq!(omitted.source(), ArchiveSource::Stdin);
        assert_eq!(
            omitted.require_file().unwrap_err().to_string(),
            "archive input from standard input is not supported by this command yet; specify the archive with --file"
        );

        let dash = ArchiveFileArgs {
            file: Some(PathBuf::from("-")),
        };
        assert!(matches!(
            dash.source(),
            ArchiveSource::File(path) if path == Path::new("-")
        ));
    }

    /// Reports which access path [`OpenArchiveSource::consume`] hands a consumer.
    struct SourceKind;

    impl SourceConsumer for SourceKind {
        type Output = &'static str;

        fn readers<R: io::Read, I: Iterator<Item = R>>(self, _: I) -> &'static str {
            "readers"
        }

        #[cfg(feature = "memmap")]
        fn bytes<'d>(self, _: impl Iterator<Item = &'d [u8]> + Send) -> &'static str {
            "bytes"
        }
    }

    #[test]
    fn stdin_source_opens_the_generic_reader() {
        assert_eq!(
            ArchiveSource::Stdin.open().unwrap().consume(SourceKind),
            "readers"
        );
    }
}
