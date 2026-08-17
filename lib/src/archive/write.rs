//! Archive writing and entry serialization.

use crate::{
    PNA_SIGNATURE,
    archive::{Archive, ArchiveHeader, SolidArchive},
    chunk::{Chunk, ChunkStreamWriter, ChunkType, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        DataKind, Entry, EntryHeader, EntryName, EntryPart, EntryWriteAttributes, NormalEntry,
        SealedEntryExt, SolidHeader, WriteCipher, WriteOption, WriteOptions, get_writer,
        get_writer_context, write_metadata_facets,
    },
    util::io::TryIntoInner,
};
use core::num::NonZeroU32;
#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
#[cfg(feature = "unstable-async")]
use futures_util::AsyncWriteExt;
use std::io::{self, Write};

/// Internal Writer type alias.
pub(crate) type InternalDataWriter<W> = CompressionWriter<CipherWriter<W>>;

/// Internal Writer type alias.
pub(crate) type InternalArchiveDataWriter<W> = InternalDataWriter<ChunkStreamWriter<W>>;

/// Writer for an entry payload, compressed and encrypted according to the given options.
pub struct EntryDataWriter<W: Write>(InternalArchiveDataWriter<W>);

impl<W: Write> Write for EntryDataWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// A writer for individual entry payloads within a solid archive.
///
/// This type is passed by mutable reference to the closure in
/// [`SolidArchive::write_file`] or [`SolidArchive::write_opaque`] and implements
/// [`Write`](std::io::Write), allowing callers to stream entry data into the
/// solid archive's shared compression and encryption pipeline.
pub struct SolidArchiveEntryDataWriter<'w, W: Write>(
    InternalArchiveDataWriter<&'w mut InternalArchiveDataWriter<W>>,
);

impl<W: Write> Write for SolidArchiveEntryDataWriter<'_, W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<W: Write> Archive<W> {
    /// Writes the archive header to the given `Write` object and returns a new [`Archive`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing header to the writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::Archive;
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::create("example.pna")?;
    /// let mut archive = Archive::write_header(file)?;
    /// archive.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn write_header(write: W) -> io::Result<Self> {
        let header = ArchiveHeader::new(0, 0, 0);
        Self::write_header_with(write, header)
    }

    #[inline]
    fn write_header_with(mut write: W, header: ArchiveHeader) -> io::Result<Self> {
        write.write_all(PNA_SIGNATURE)?;
        crate::io::write_chunk(&mut write, (ChunkType::AHED, header.to_bytes()))?;
        Ok(Self::new(write, header))
    }

    /// Writes a regular file as a normal entry into the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry, or if the closure returns an error.
    /// If this method returns an error, the archive may contain a partial entry
    /// and must be discarded without further use.
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, Metadata, WriteOptions};
    /// # use std::error::Error;
    /// use std::fs;
    /// use std::io::{self, prelude::*};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let file = fs::File::create("foo.pna")?;
    /// let mut archive = Archive::write_header(file)?;
    /// archive.write_file(
    ///     "bar.txt".into(),
    ///     Metadata::new(),
    ///     WriteOptions::builder().build(),
    ///     |writer| writer.write_all(b"text"),
    /// )?;
    /// archive.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn write_file<F>(
        &mut self,
        name: EntryName,
        attributes: impl Into<EntryWriteAttributes>,
        option: impl WriteOption,
        f: F,
    ) -> io::Result<()>
    where
        F: FnOnce(&mut EntryDataWriter<&mut W>) -> io::Result<()>,
    {
        write_stream_entry(
            &mut self.inner,
            name,
            DataKind::FILE,
            attributes.into(),
            option,
            self.max_chunk_size,
            |w| {
                let mut w = EntryDataWriter(w);
                f(&mut w)?;
                Ok(w.0)
            },
        )
    }

    /// Writes an opaque entry payload with the declared data kind.
    ///
    /// No validation is performed between `kind` and the payload written by
    /// the closure. Prefer the kind-specific builder or [`Self::write_file`]
    /// for data kinds defined by the PNA specification; this method is a
    /// low-level escape hatch for private, reserved, or experimental kinds.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry, or if
    /// the closure returns an error. If this method returns an error, the
    /// archive may contain a partial entry and must be discarded without
    /// further use.
    #[inline]
    pub fn write_opaque<F>(
        &mut self,
        name: EntryName,
        kind: DataKind,
        attributes: impl Into<EntryWriteAttributes>,
        option: impl WriteOption,
        f: F,
    ) -> io::Result<()>
    where
        F: FnOnce(&mut EntryDataWriter<&mut W>) -> io::Result<()>,
    {
        write_stream_entry(
            &mut self.inner,
            name,
            kind,
            attributes.into(),
            option,
            self.max_chunk_size,
            |w| {
                let mut w = EntryDataWriter(w);
                f(&mut w)?;
                Ok(w.0)
            },
        )
    }

    /// Adds a new entry to the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, FileEntryBuilder, WriteOptions};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::create("example.pna")?;
    /// let mut archive = Archive::write_header(file)?;
    /// archive.add_entry(
    ///     FileEntryBuilder::new_with_options("example.txt".into(), WriteOptions::builder().build())?
    ///         .build()?,
    /// )?;
    /// archive.finalize()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<usize> {
        entry.write_in(&mut self.inner)
    }

    /// Adds a part of an entry to the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry part.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use libpna::{Archive, EntryPart, FileEntryBuilder, WriteOptions};
    /// # use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1_file = File::create("example.part1.pna")?;
    /// let mut archive_part1 = Archive::write_header(part1_file)?;
    /// let entry =
    ///     FileEntryBuilder::new_with_options("example.txt".into(), WriteOptions::builder().build())?
    ///         .build()?;
    /// archive_part1.add_entry_part(EntryPart::from(entry))?;
    ///
    /// let part2_file = File::create("example.part2.pna")?;
    /// let archive_part2 = archive_part1.split_to_next_archive(part2_file)?;
    /// archive_part2.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn add_entry_part<T>(&mut self, entry_part: EntryPart<T>) -> io::Result<usize>
    where
        RawChunk<T>: Chunk,
    {
        let mut written_len = 0;
        for chunk in entry_part.0 {
            written_len += crate::io::write_chunk(&mut self.inner, chunk)?;
        }
        Ok(written_len)
    }

    #[inline]
    fn add_next_archive_marker(&mut self) -> io::Result<usize> {
        crate::io::write_chunk(&mut self.inner, (ChunkType::ANXT, []))
    }

    /// Splits to the next archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while splitting to the next archive.
    ///
    /// # Examples
    /// ```no_run
    /// # use libpna::{Archive, EntryPart, FileEntryBuilder, WriteOptions};
    /// # use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1_file = File::create("example.part1.pna")?;
    /// let mut archive_part1 = Archive::write_header(part1_file)?;
    /// let entry =
    ///     FileEntryBuilder::new_with_options("example.txt".into(), WriteOptions::builder().build())?
    ///         .build()?;
    /// archive_part1.add_entry_part(EntryPart::from(entry))?;
    ///
    /// let part2_file = File::create("example.part2.pna")?;
    /// let archive_part2 = archive_part1.split_to_next_archive(part2_file)?;
    /// archive_part2.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn split_to_next_archive<OW: Write>(mut self, writer: OW) -> io::Result<Archive<OW>> {
        let next_archive_number = self.header.archive_number + 1;
        let header = ArchiveHeader::new(0, 0, next_archive_number);
        let max_chunk_size = self.max_chunk_size;
        self.add_next_archive_marker()?;
        self.finalize()?;
        let mut archive = Archive::write_header_with(writer, header)?;
        archive.max_chunk_size = max_chunk_size;
        Ok(archive)
    }

    /// Writes the end-of-archive marker and finalizes the archive.
    ///
    /// Marks that the PNA archive contains no more entries.
    /// Normally, a PNA archive reader will continue reading entries in the hope that the entry exists until it encounters this end marker.
    /// This end marker should always be recorded at the end of the file unless there is a special reason to do so.
    ///
    /// # Errors
    /// Returns an error if writing the end-of-archive marker fails.
    ///
    /// # Examples
    /// Creates an empty archive.
    /// ```no_run
    /// # use std::io;
    /// # use std::fs::File;
    /// # use libpna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = File::create("foo.pna")?;
    /// let mut archive = Archive::write_header(file)?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "archive is not complete until finalize succeeds"]
    pub fn finalize(mut self) -> io::Result<W> {
        crate::io::write_chunk(&mut self.inner, (ChunkType::AEND, []))?;
        Ok(self.inner)
    }
}

#[cfg(feature = "unstable-async")]
impl<W: AsyncWrite + Unpin> Archive<W> {
    /// Writes the archive header to the given object and returns a new [`Archive`].
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing header to the writer.
    #[inline]
    pub async fn write_header_async(write: W) -> io::Result<Self> {
        let header = ArchiveHeader::new(0, 0, 0);
        Self::write_header_with_async(write, header).await
    }

    #[inline]
    async fn write_header_with_async(mut write: W, header: ArchiveHeader) -> io::Result<Self> {
        write.write_all(PNA_SIGNATURE).await?;
        crate::async_io::write_chunk(&mut write, (ChunkType::AHED, header.to_bytes())).await?;
        Ok(Self::new(write, header))
    }

    /// Adds a new entry to the archive.
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
    #[inline]
    pub async fn add_entry_async(&mut self, entry: impl Entry) -> io::Result<usize> {
        let mut written_len = 0;
        for chunk in entry.into_chunks() {
            written_len += crate::async_io::write_chunk(&mut self.inner, chunk).await?;
        }
        Ok(written_len)
    }

    /// Writes the end-of-archive marker and finalizes the archive.
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the end-of-archive marker fails.
    #[inline]
    pub async fn finalize_async(mut self) -> io::Result<W> {
        crate::async_io::write_chunk(&mut self.inner, (ChunkType::AEND, [])).await?;
        Ok(self.inner)
    }
}

impl<W: Write> Archive<W> {
    /// Writes the archive header and creates a new [`SolidArchive`] with the specified
    /// compression and encryption options for solid mode.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing header to the writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, WriteOptions};
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let option = WriteOptions::builder().build();
    /// let file = File::create("example.pna")?;
    /// let mut archive = Archive::write_solid_header(file, option)?;
    /// archive.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn write_solid_header(write: W, option: impl WriteOption) -> io::Result<SolidArchive<W>> {
        let archive = Self::write_header(write)?;
        archive.into_solid_archive(option)
    }

    #[inline]
    fn into_solid_archive(mut self, option: impl WriteOption) -> io::Result<SolidArchive<W>> {
        let header = SolidHeader::new(
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
        );
        let context = get_writer_context(option, ChunkType::SHED, &header.to_bytes())?;

        crate::io::write_chunk(&mut self.inner, (ChunkType::SHED, header.to_bytes()))?;
        if let Some(WriteCipher { context: c, .. }) = &context.cipher {
            crate::io::write_chunk(&mut self.inner, (ChunkType::PHSF, c.phsf.as_bytes()))?;
        }
        self.inner.flush()?;
        let max_chunk_size = self.max_chunk_size;
        let mut writer = ChunkStreamWriter::new(ChunkType::SDAT, self.inner, max_chunk_size);
        if let Some(WriteCipher { context: c, .. }) = &context.cipher {
            writer.write_all(&c.prefix_bytes())?;
        }
        let writer = get_writer(writer, &context)?;

        Ok(SolidArchive {
            archive_header: self.header,
            inner: writer,
            max_chunk_size: None,
        })
    }
}

impl<W: Write> SolidArchive<W> {
    /// Adds a new entry to the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, FileEntryBuilder, WriteOptions};
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let option = WriteOptions::builder().build();
    /// let file = File::create("example.pna")?;
    /// let mut archive = Archive::write_solid_header(file, option)?;
    /// archive.add_entry(FileEntryBuilder::new("example.txt".into())?.build()?)?;
    /// archive.finalize()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn add_entry<T>(&mut self, entry: NormalEntry<T>) -> io::Result<usize>
    where
        NormalEntry<T>: Entry,
    {
        entry.write_in(&mut self.inner)
    }

    /// Writes a regular file as a solid entry into the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry, or if the closure returns an error.
    /// If this method returns an error, the solid archive may contain a partial
    /// entry and must be discarded without further use.
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, Metadata, WriteOptions};
    /// # use std::error::Error;
    /// use std::fs;
    /// use std::io::{self, prelude::*};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let file = fs::File::create("foo.pna")?;
    /// let option = WriteOptions::builder().build();
    /// let mut archive = Archive::write_solid_header(file, option)?;
    /// archive.write_file("bar.txt".into(), Metadata::new(), |writer| {
    ///     writer.write_all(b"text")
    /// })?;
    /// archive.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn write_file<F>(
        &mut self,
        name: EntryName,
        attributes: impl Into<EntryWriteAttributes>,
        f: F,
    ) -> io::Result<()>
    where
        F: FnOnce(&mut SolidArchiveEntryDataWriter<W>) -> io::Result<()>,
    {
        let option = WriteOptions::store();
        write_stream_entry(
            &mut self.inner,
            name,
            DataKind::FILE,
            attributes.into(),
            option,
            self.max_chunk_size,
            |w| {
                let mut w = SolidArchiveEntryDataWriter(w);
                f(&mut w)?;
                Ok(w.0)
            },
        )
    }

    /// Writes an opaque entry payload into the solid archive.
    ///
    /// The inner entry always uses STORE; compression and encryption are
    /// provided only by the outer solid stream. No validation is performed
    /// between `kind` and the payload. Prefer kind-specific APIs for data kinds
    /// defined by the PNA specification.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry, or if
    /// the closure returns an error. If this method returns an error, the solid
    /// archive may contain a partial entry and must be discarded without
    /// further use.
    #[inline]
    pub fn write_opaque<F>(
        &mut self,
        name: EntryName,
        kind: DataKind,
        attributes: impl Into<EntryWriteAttributes>,
        f: F,
    ) -> io::Result<()>
    where
        F: FnOnce(&mut SolidArchiveEntryDataWriter<W>) -> io::Result<()>,
    {
        write_stream_entry(
            &mut self.inner,
            name,
            kind,
            attributes.into(),
            WriteOptions::store(),
            self.max_chunk_size,
            |w| {
                let mut w = SolidArchiveEntryDataWriter(w);
                f(&mut w)?;
                Ok(w.0)
            },
        )
    }

    /// Sets the maximum chunk size for entry data (FDAT) written via
    /// [`write_file()`](SolidArchive::write_file) or
    /// [`write_opaque()`](SolidArchive::write_opaque).
    ///
    /// This controls the inner FDAT chunk splitting for individual entries within
    /// the solid stream. The outer SDAT chunk size is fixed when `SolidArchive` is
    /// constructed and cannot be changed afterward. To control the outer SDAT chunk
    /// size, call [`Archive::set_max_chunk_size`] before
    /// [`write_solid_header()`](Archive::write_solid_header).
    ///
    /// Pre-built entries added via [`add_entry()`](SolidArchive::add_entry) use their own
    /// chunk size configured through [`FileEntryBuilder::max_chunk_size()`](crate::FileEntryBuilder::max_chunk_size).
    #[inline]
    pub fn set_max_file_chunk_size(&mut self, size: NonZeroU32) {
        self.max_chunk_size = Some(size);
    }

    /// Writes the end-of-archive marker and finalizes the archive.
    ///
    /// Marks that the PNA archive contains no more entries.
    /// Normally, a PNA archive reader will continue reading entries in the hope that the entry exists until it encounters this end marker.
    /// This end marker should always be recorded at the end of the file unless there is a special reason to do so.
    ///
    /// # Errors
    /// Returns an error if writing the end-of-archive marker fails.
    ///
    /// # Examples
    /// Creates an empty archive.
    /// ```no_run
    /// use libpna::{Archive, WriteOptions};
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let option = WriteOptions::builder().build();
    /// let file = File::create("example.pna")?;
    /// let mut archive = Archive::write_solid_header(file, option)?;
    /// archive.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "archive is not complete until finalize succeeds"]
    pub fn finalize(self) -> io::Result<W> {
        let archive = self.finalize_solid_entry()?;
        archive.finalize()
    }

    #[inline]
    fn finalize_solid_entry(mut self) -> io::Result<Archive<W>> {
        self.inner.flush()?;
        let mut inner = self.inner.try_into_inner()?.try_into_inner()?.into_inner();
        crate::io::write_chunk(&mut inner, (ChunkType::SEND, []))?;
        Ok(Archive::new(inner, self.archive_header))
    }
}

pub(crate) fn write_stream_entry<W, F>(
    inner: &mut W,
    name: EntryName,
    kind: DataKind,
    attributes: EntryWriteAttributes,
    option: impl WriteOption,
    max_chunk_size: Option<NonZeroU32>,
    f: F,
) -> io::Result<()>
where
    W: Write,
    F: FnOnce(InternalArchiveDataWriter<&mut W>) -> io::Result<InternalArchiveDataWriter<&mut W>>,
{
    let EntryWriteAttributes {
        metadata,
        extra_chunks,
    } = attributes;
    let header = EntryHeader::new_with_options(
        kind,
        option.compression(),
        option.encryption(),
        option.cipher_mode(),
        name,
    );
    let header_bytes = header.to_bytes();
    crate::io::write_chunk(inner, (ChunkType::FHED, &header_bytes))?;
    for chunk in extra_chunks {
        crate::io::write_chunk(inner, chunk)?;
    }
    write_metadata_facets(inner, &metadata)?;
    let context = get_writer_context(option, ChunkType::FHED, &header_bytes)?;
    if let Some(WriteCipher { context: c, .. }) = &context.cipher {
        crate::io::write_chunk(inner, (ChunkType::PHSF, c.phsf.as_bytes()))?;
    }
    let inner = {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, inner, max_chunk_size);
        if let Some(WriteCipher { context: c, .. }) = &context.cipher {
            writer.write_all(&c.prefix_bytes())?;
        }
        let writer = get_writer(writer, &context)?;
        let mut writer = f(writer)?;
        writer.flush()?;
        writer.try_into_inner()?.try_into_inner()?.into_inner()
    };
    crate::io::write_chunk(inner, (ChunkType::FEND, Vec::<u8>::new()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CipherMode, Compression, Encryption, HashAlgorithm, LinkTargetType, Metadata, ReadOptions,
    };
    use std::io::Read;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn encode() {
        let writer = Archive::write_header(Vec::new()).expect("failed to write header");
        let file = writer.finalize().expect("failed to finalize");
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }

    #[test]
    fn archive_write_file_entry() {
        let option = WriteOptions::builder().build();
        let mut writer = Archive::write_header(Vec::new()).expect("failed to write header");
        writer
            .write_file(
                EntryName::from_lossy("text.txt"),
                Metadata::new(),
                option,
                |writer| writer.write_all(b"text"),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");
        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let mut entries = reader.entries_with_options(&ReadOptions::builder().build());
        let entry = entries
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        let mut data_reader = entry
            .reader(ReadOptions::builder().build())
            .expect("failed to read entry data");
        let mut data = Vec::new();
        data_reader
            .read_to_end(&mut data)
            .expect("failed to read data");
        assert_eq!(&data[..], b"text");
    }

    #[test]
    fn archive_write_file_accepts_attributes_without_generating_file_size() {
        let extra_type = ChunkType::private(*b"exTr").unwrap();
        let extra = RawChunk::from_data(extra_type, b"extra".to_vec());
        let mut attributes = EntryWriteAttributes::new(
            Metadata::new().with_link_target_type(Some(LinkTargetType::Directory)),
        );
        attributes.add_extra_chunk(extra);
        let payload = b"streamed".to_vec();

        let mut writer = Archive::write_header(Vec::new()).unwrap();
        writer
            .write_file(
                EntryName::from_lossy("text.txt"),
                attributes,
                WriteOptions::store(),
                move |writer| {
                    let payload = payload;
                    writer.write_all(&payload)
                },
            )
            .unwrap();
        let archive = writer.finalize().unwrap();

        assert_eq!(count_chunks(&archive, ChunkType::fSIZ), 0);
        let chunk_types = crate::chunk::read_chunks_from_slice(&archive)
            .unwrap()
            .map(|chunk| chunk.unwrap().ty())
            .collect::<Vec<_>>();
        let header_index = chunk_types
            .iter()
            .position(|&ty| ty == ChunkType::FHED)
            .unwrap();
        assert_eq!(chunk_types[header_index + 1], extra_type);
        assert_eq!(chunk_types[header_index + 2], ChunkType::fLTP);
        let mut reader = Archive::read_header(archive.as_slice()).unwrap();
        let entry = reader.entries().skip_solid().next().unwrap().unwrap();
        assert_eq!(entry.metadata().raw_file_size(), None);
        assert_eq!(
            entry.metadata().link_target_type(),
            Some(LinkTargetType::Directory)
        );
        assert_eq!(entry.extra_chunks().len(), 1);
        assert_eq!(entry.extra_chunks()[0].ty(), extra_type);
        assert_eq!(entry.extra_chunks()[0].data(), b"extra");
        let mut data = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"streamed");
    }

    #[test]
    fn archive_write_opaque_round_trips_private_kind() {
        let kind = DataKind::new_private(200).unwrap();
        let extra_type = ChunkType::private(*b"opAq").unwrap();
        let mut attributes = EntryWriteAttributes::new(
            Metadata::new().with_modified(Some(crate::Duration::seconds(42))),
        );
        attributes.add_extra_chunk(RawChunk::from_data(extra_type, b"metadata".to_vec()));
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        writer
            .write_opaque(
                EntryName::from_lossy("private"),
                kind,
                attributes,
                WriteOptions::store(),
                |writer| writer.write_all(b"opaque"),
            )
            .unwrap();
        let archive = writer.finalize().unwrap();

        let mut reader = Archive::read_header(archive.as_slice()).unwrap();
        let entry = reader.entries().skip_solid().next().unwrap().unwrap();
        assert_eq!(entry.header().data_kind(), kind);
        assert_eq!(
            entry.metadata().modified(),
            Some(crate::Duration::seconds(42))
        );
        assert_eq!(entry.extra_chunks()[0].ty(), extra_type);
        let mut data = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"opaque");
    }

    #[test]
    fn archive_write_opaque_accepts_standard_kind() {
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        writer
            .write_opaque(
                EntryName::from_lossy("declared-directory"),
                DataKind::DIRECTORY,
                Metadata::new(),
                WriteOptions::store(),
                |writer| writer.write_all(b"unchecked payload"),
            )
            .unwrap();
        let archive = writer.finalize().unwrap();

        let mut reader = Archive::read_header(archive.as_slice()).unwrap();
        let entry = reader.entries().skip_solid().next().unwrap().unwrap();
        assert_eq!(entry.header().data_kind(), DataKind::DIRECTORY);
        let mut data = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"unchecked payload");
    }

    #[test]
    fn solid_archive_write_opaque_uses_store_for_inner_entry() {
        let kind = DataKind::new_private(201).unwrap();
        let mut writer = Archive::write_solid_header(
            Vec::new(),
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .build(),
        )
        .unwrap();
        writer
            .write_opaque(
                EntryName::from_lossy("private"),
                kind,
                Metadata::new(),
                |writer| writer.write_all(b"solid opaque"),
            )
            .unwrap();
        let archive = writer.finalize().unwrap();

        let mut reader = Archive::read_header(archive.as_slice()).unwrap();
        let entry = reader
            .entries()
            .extract_solid_entries(&ReadOptions::builder().build())
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(entry.header().data_kind(), kind);
        assert_eq!(entry.header().compression(), Compression::NO);
        let mut data = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"solid opaque");
    }

    fn owner_facet_metadata() -> Metadata {
        Metadata::new()
            .with_owner_uid(Some(crate::OwnerUid::from(1000)))
            .with_owner_gid(Some(crate::OwnerGid::from(100)))
            .with_owner_user_name(Some(crate::OwnerUserName::new("alice").unwrap()))
            .with_owner_group_name(Some(crate::OwnerGroupName::new("devs").unwrap()))
            .with_owner_user_sid(Some(crate::OwnerUserSid::new("S-1-1").unwrap()))
            .with_owner_group_sid(Some(crate::OwnerGroupSid::new("S-1-2").unwrap()))
            .with_permission_mode(Some(crate::PermissionMode::from(0o750)))
    }

    fn assert_owner_facet_metadata(metadata: &Metadata) {
        assert_eq!(metadata.owner_uid().map(|v| v.get()), Some(1000));
        assert_eq!(metadata.owner_gid().map(|v| v.get()), Some(100));
        assert_eq!(
            metadata.owner_user_name().map(|v| v.as_str()),
            Some("alice")
        );
        assert_eq!(
            metadata.owner_group_name().map(|v| v.as_str()),
            Some("devs")
        );
        assert_eq!(metadata.owner_user_sid().map(|v| v.as_str()), Some("S-1-1"));
        assert_eq!(
            metadata.owner_group_sid().map(|v| v.as_str()),
            Some("S-1-2")
        );
        assert_eq!(metadata.permission_mode().map(|v| v.get()), Some(0o750));
    }

    #[test]
    fn archive_write_file_entry_preserves_owner_facets() {
        let mut writer = Archive::write_header(Vec::new()).expect("failed to write header");
        writer
            .write_file(
                EntryName::from_lossy("text.txt"),
                owner_facet_metadata(),
                WriteOptions::store(),
                |writer| writer.write_all(b"text"),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");
        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let entry = reader
            .entries()
            .skip_solid()
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        assert_owner_facet_metadata(entry.metadata());
    }

    #[test]
    fn solid_write_file_entry() {
        let option = WriteOptions::builder().build();
        let mut writer =
            Archive::write_solid_header(Vec::new(), option).expect("failed to write header");
        writer
            .write_file(
                EntryName::from_lossy("text.txt"),
                Metadata::new(),
                |writer| writer.write_all(b"text"),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");
        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let mut entries = reader.entries_with_options(&ReadOptions::builder().build());
        let entry = entries
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        let mut data_reader = entry
            .reader(ReadOptions::builder().build())
            .expect("failed to read entry data");
        let mut data = Vec::new();
        data_reader
            .read_to_end(&mut data)
            .expect("failed to read data");
        assert_eq!(&data[..], b"text");
    }

    #[test]
    fn solid_write_file_entry_preserves_owner_facets() {
        let mut writer = Archive::write_solid_header(Vec::new(), WriteOptions::store())
            .expect("failed to write header");
        writer
            .write_file(
                EntryName::from_lossy("text.txt"),
                owner_facet_metadata(),
                |writer| writer.write_all(b"text"),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");
        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let entry = reader
            .entries()
            .extract_solid_entries(&ReadOptions::builder().build())
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        assert_owner_facet_metadata(entry.metadata());
    }

    fn count_chunks(archive: &[u8], ty: ChunkType) -> usize {
        crate::chunk::read_chunks_from_slice(archive)
            .unwrap()
            .filter(|c| c.as_ref().unwrap().ty() == ty)
            .count()
    }

    fn assert_chunk_sizes_at_most(archive: &[u8], ty: ChunkType, max: usize) {
        let sizes = crate::chunk::read_chunks_from_slice(archive)
            .unwrap()
            .filter_map(|chunk| {
                let chunk = chunk.unwrap();
                (chunk.ty() == ty).then(|| chunk.data().len())
            })
            .collect::<Vec<_>>();
        assert!(!sizes.is_empty(), "expected at least one {ty:?} chunk");
        assert!(
            sizes.iter().all(|&size| size <= max),
            "{ty:?} chunk sizes {sizes:?} must all be <= {max}"
        );
    }

    fn gcm_write_options() -> WriteOptions {
        WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::GCM)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .build()
    }

    #[test]
    fn archive_write_file_gcm_header_respects_max_chunk_size() {
        let mut writer = Archive::write_header(Vec::new()).expect("failed to write header");
        writer.set_max_chunk_size(NonZeroU32::new(8).unwrap());
        writer
            .write_file(
                EntryName::from_lossy("file"),
                Metadata::new(),
                gcm_write_options(),
                |writer| writer.write_all(b"x"),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");

        assert_chunk_sizes_at_most(&file, ChunkType::FDAT, 8);

        let mut reader = Archive::read_header(file.as_slice()).unwrap();
        let entry = reader.entries().skip_solid().next().unwrap().unwrap();
        let mut data = entry
            .reader(ReadOptions::with_password(Some("password")))
            .unwrap();
        let mut plain = Vec::new();
        data.read_to_end(&mut plain).unwrap();
        assert_eq!(plain, b"x");
    }

    #[test]
    fn solid_archive_gcm_header_respects_max_chunk_size() {
        let mut archive = Archive::write_header(Vec::new()).expect("failed to write header");
        archive.set_max_chunk_size(NonZeroU32::new(8).unwrap());
        let mut writer = archive
            .into_solid_archive(gcm_write_options())
            .expect("failed to create solid archive");
        writer
            .write_file(EntryName::from_lossy("file"), Metadata::new(), |writer| {
                writer.write_all(b"x")
            })
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");

        assert_chunk_sizes_at_most(&file, ChunkType::SDAT, 8);

        let mut reader = Archive::read_header(file.as_slice()).unwrap();
        let entry = reader
            .entries()
            .extract_solid_entries(&ReadOptions::with_password(Some("password")))
            .next()
            .unwrap()
            .unwrap();
        let mut data = entry.reader(ReadOptions::builder().build()).unwrap();
        let mut plain = Vec::new();
        data.read_to_end(&mut plain).unwrap();
        assert_eq!(plain, b"x");
    }

    #[test]
    fn archive_write_file_with_max_chunk_size() {
        let option = WriteOptions::builder().build();
        let mut writer = Archive::write_header(Vec::new()).expect("failed to write header");
        writer.set_max_chunk_size(NonZeroU32::new(8).unwrap());
        let large_data = b"abcdefghijklmnopqrstuvwxyz";
        writer
            .write_file(
                EntryName::from_lossy("large.txt"),
                Metadata::new(),
                option,
                |writer| writer.write_all(large_data),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");

        let fdat_count = count_chunks(&file, ChunkType::FDAT);
        assert!(
            fdat_count >= 4,
            "26 bytes with max_chunk_size=8 should produce at least 4 FDAT chunks, got {fdat_count}"
        );

        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let mut entries = reader.entries_with_options(&ReadOptions::builder().build());
        let entry = entries
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        let mut data_reader = entry
            .reader(ReadOptions::builder().build())
            .expect("failed to read entry data");
        let mut data = Vec::new();
        data_reader
            .read_to_end(&mut data)
            .expect("failed to read data");
        assert_eq!(&data[..], large_data);
    }

    #[test]
    fn solid_archive_write_file_with_max_chunk_size() {
        let option = WriteOptions::builder().build();
        let mut archive = Archive::write_header(Vec::new()).expect("failed to write header");
        archive.set_max_chunk_size(NonZeroU32::new(8).unwrap());
        let mut writer = archive
            .into_solid_archive(option)
            .expect("failed to create solid archive");
        let large_data = b"abcdefghijklmnopqrstuvwxyz";
        writer
            .write_file(
                EntryName::from_lossy("large.txt"),
                Metadata::new(),
                |writer| writer.write_all(large_data),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");

        // Outer SDAT chunks should be split by max_chunk_size
        let sdat_count = count_chunks(&file, ChunkType::SDAT);
        assert!(
            sdat_count >= 2,
            "outer SDAT should be split with max_chunk_size=8, got {sdat_count}"
        );

        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let mut entries = reader.entries_with_options(&ReadOptions::builder().build());
        let entry = entries
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        let mut data_reader = entry
            .reader(ReadOptions::builder().build())
            .expect("failed to read entry data");
        let mut data = Vec::new();
        data_reader
            .read_to_end(&mut data)
            .expect("failed to read data");
        assert_eq!(&data[..], large_data);
    }

    #[test]
    fn solid_archive_set_max_file_chunk_size_after_creation() {
        let option = WriteOptions::builder().build();
        let mut writer =
            Archive::write_solid_header(Vec::new(), option).expect("failed to write header");
        writer.set_max_file_chunk_size(NonZeroU32::new(8).unwrap());
        let large_data = b"abcdefghijklmnopqrstuvwxyz";
        writer
            .write_file(
                EntryName::from_lossy("large.txt"),
                Metadata::new(),
                |writer| writer.write_all(large_data),
            )
            .expect("failed to write");
        let file = writer.finalize().expect("failed to finalize");
        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let mut entries = reader.entries_with_options(&ReadOptions::builder().build());
        let entry = entries
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        let mut data_reader = entry
            .reader(ReadOptions::builder().build())
            .expect("failed to read entry data");
        let mut data = Vec::new();
        data_reader
            .read_to_end(&mut data)
            .expect("failed to read data");
        assert_eq!(&data[..], large_data);
    }

    #[test]
    fn split_to_next_archive_preserves_max_chunk_size() {
        let option = WriteOptions::builder().build();
        let mut writer = Archive::write_header(Vec::new()).expect("failed to write header");
        writer.set_max_chunk_size(NonZeroU32::new(8).unwrap());

        let next_writer = writer
            .split_to_next_archive(Vec::new())
            .expect("failed to split");
        let large_data = b"abcdefghijklmnopqrstuvwxyz";
        let mut next_writer = next_writer;
        next_writer
            .write_file(
                EntryName::from_lossy("large.txt"),
                Metadata::new(),
                option,
                |writer| writer.write_all(large_data),
            )
            .expect("failed to write");
        let file = next_writer.finalize().expect("failed to finalize");

        let fdat_count = count_chunks(&file, ChunkType::FDAT);
        assert!(
            fdat_count >= 4,
            "max_chunk_size should be preserved across split, got {fdat_count} FDAT chunks"
        );

        let mut reader = Archive::read_header(&file[..]).expect("failed to read archive");
        let mut entries = reader.entries_with_options(&ReadOptions::builder().build());
        let entry = entries
            .next()
            .expect("failed to get entry")
            .expect("failed to read entry");
        let mut data_reader = entry
            .reader(ReadOptions::builder().build())
            .expect("failed to read entry data");
        let mut data = Vec::new();
        data_reader
            .read_to_end(&mut data)
            .expect("failed to read data");
        assert_eq!(&data[..], large_data);
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn encode_async() {
        use tokio_util::compat::TokioAsyncWriteCompatExt;

        let archive_bytes = {
            let file = Vec::new().compat_write();
            let writer = Archive::write_header_async(file).await.unwrap();
            writer.finalize_async().await.unwrap().into_inner()
        };
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(archive_bytes.as_slice(), expected.as_slice());
    }

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn add_entry_async_matches_sync_encoding() {
        use crate::FileEntryBuilder;
        use tokio_util::compat::TokioAsyncWriteCompatExt;

        let mut builder =
            FileEntryBuilder::new_with_options("file".into(), WriteOptions::store()).unwrap();
        std::io::Write::write_all(&mut builder, b"entry data").unwrap();
        let entry = builder.build().unwrap();

        let sync_bytes = {
            let mut archive = Archive::write_header(Vec::new()).unwrap();
            archive.add_entry(entry.clone()).unwrap();
            archive.finalize().unwrap()
        };
        let async_bytes = {
            let file = Vec::new().compat_write();
            let mut archive = Archive::write_header_async(file).await.unwrap();
            archive.add_entry_async(entry).await.unwrap();
            archive.finalize_async().await.unwrap().into_inner()
        };

        assert_eq!(async_bytes, sync_bytes);
    }
}
