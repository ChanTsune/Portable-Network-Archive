use crate::{
    archive::{Archive, ArchiveHeader, PNA_HEADER, SolidArchive},
    chunk::{Chunk, ChunkExt, ChunkStreamWriter, ChunkType, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        Entry, EntryHeader, EntryName, EntryPart, Metadata, NormalEntry, SealedEntryExt,
        SolidHeader, WriteCipher, WriteOption, WriteOptions, get_writer, get_writer_context,
    },
    io::TryIntoInner,
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

/// Writer that compresses and encrypts according to the given options.
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
    /// Writes the archive header to the given `Write` object and return a new [Archive].
    ///
    /// # Arguments
    ///
    /// * `write` - The [Write] object to write the header to.
    ///
    /// # Returns
    ///
    /// A new [`io::Result<Archive<W>>`]
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
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing header to the writer.
    #[inline]
    pub fn write_header(write: W) -> io::Result<Self> {
        let header = ArchiveHeader::new(0, 0, 0);
        Self::write_header_with(write, header)
    }

    #[inline]
    fn write_header_with(mut write: W, header: ArchiveHeader) -> io::Result<Self> {
        write.write_all(PNA_HEADER)?;
        (ChunkType::AHED, header.to_bytes()).write_chunk_in(&mut write)?;
        Ok(Self::new(write, header))
    }

    /// Writes a regular file as a normal entry into the archive.
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
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry, or if the closure returns an error.
    #[inline]
    pub fn write_file<F>(
        &mut self,
        name: EntryName,
        metadata: Metadata,
        option: impl WriteOption,
        mut f: F,
    ) -> io::Result<()>
    where
        F: FnMut(&mut EntryDataWriter<&mut W>) -> io::Result<()>,
    {
        write_file_entry(
            &mut self.inner,
            name,
            metadata,
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
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, EntryBuilder, WriteOptions};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::create("example.pna")?;
    /// let mut archive = Archive::write_header(file)?;
    /// archive.add_entry(
    ///     EntryBuilder::new_file("example.txt".into(), WriteOptions::builder().build())?.build()?,
    /// )?;
    /// archive.finalize()?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
    #[inline]
    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<usize> {
        entry.write_in(&mut self.inner)
    }

    /// Adds a part of an entry to the archive.
    ///
    /// # Arguments
    ///
    /// * `entry_part` - The part of an entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use libpna::{Archive, EntryBuilder, EntryPart, WriteOptions};
    /// # use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1_file = File::create("example.part1.pna")?;
    /// let mut archive_part1 = Archive::write_header(part1_file)?;
    /// let entry =
    ///     EntryBuilder::new_file("example.txt".into(), WriteOptions::builder().build())?.build()?;
    /// archive_part1.add_entry_part(EntryPart::from(entry))?;
    ///
    /// let part2_file = File::create("example.part2.pna")?;
    /// let archive_part2 = archive_part1.split_to_next_archive(part2_file)?;
    /// archive_part2.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry part.
    #[inline]
    pub fn add_entry_part<T>(&mut self, entry_part: EntryPart<T>) -> io::Result<usize>
    where
        RawChunk<T>: Chunk,
    {
        let mut written_len = 0;
        for chunk in entry_part.0 {
            written_len += chunk.write_chunk_in(&mut self.inner)?;
        }
        Ok(written_len)
    }

    #[inline]
    fn add_next_archive_marker(&mut self) -> io::Result<usize> {
        (ChunkType::ANXT, []).write_chunk_in(&mut self.inner)
    }

    /// Split to the next archive.
    ///
    /// # Examples
    /// ```no_run
    /// # use libpna::{Archive, EntryBuilder, EntryPart, WriteOptions};
    /// # use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let part1_file = File::create("example.part1.pna")?;
    /// let mut archive_part1 = Archive::write_header(part1_file)?;
    /// let entry =
    ///     EntryBuilder::new_file("example.txt".into(), WriteOptions::builder().build())?.build()?;
    /// archive_part1.add_entry_part(EntryPart::from(entry))?;
    ///
    /// let part2_file = File::create("example.part2.pna")?;
    /// let archive_part2 = archive_part1.split_to_next_archive(part2_file)?;
    /// archive_part2.finalize()?;
    /// #    Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while splitting to the next archive.
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
    ///
    /// # Errors
    /// Returns an error if writing the end-of-archive marker fails.
    #[inline]
    #[must_use = "archive is not complete until finalize succeeds"]
    pub fn finalize(mut self) -> io::Result<W> {
        (ChunkType::AEND, []).write_chunk_in(&mut self.inner)?;
        Ok(self.inner)
    }
}

#[cfg(feature = "unstable-async")]
impl<W: AsyncWrite + Unpin> Archive<W> {
    /// Writes the archive header to the given object and return a new [Archive].
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
        write.write_all(PNA_HEADER).await?;
        let mut chunk_writer = crate::chunk::ChunkWriter::new(&mut write);
        chunk_writer
            .write_chunk_async((ChunkType::AHED, header.to_bytes()))
            .await?;
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
        let mut bytes = Vec::new();
        entry.write_in(&mut bytes)?;
        self.inner.write_all(&bytes).await?;
        Ok(bytes.len())
    }

    /// Writes the end-of-archive marker and finalizes the archive.
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the end-of-archive marker fails.
    #[inline]
    pub async fn finalize_async(mut self) -> io::Result<W> {
        let mut chunk_writer = crate::chunk::ChunkWriter::new(&mut self.inner);
        chunk_writer
            .write_chunk_async((ChunkType::AEND, []))
            .await?;
        Ok(self.inner)
    }
}

impl<W: Write> Archive<W> {
    /// Writes the archive header to the given `Write` object and return a new [SolidArchive].
    ///
    /// # Arguments
    ///
    /// * `write` - The [Write] object to write the header to.
    /// * `option` - The [WriteOptions] object of a solid mode option.
    ///
    /// # Returns
    ///
    /// A new [`io::Result<SolidArchive<W>>`]
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
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing header to the writer.
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
        let context = get_writer_context(option)?;

        (ChunkType::SHED, header.to_bytes()).write_chunk_in(&mut self.inner)?;
        if let Some(WriteCipher { context: c, .. }) = &context.cipher {
            (ChunkType::PHSF, c.phsf.as_bytes()).write_chunk_in(&mut self.inner)?;
            (ChunkType::SDAT, c.iv.as_slice()).write_chunk_in(&mut self.inner)?;
        }
        self.inner.flush()?;
        let max_chunk_size = self.max_chunk_size;
        let writer = get_writer(
            ChunkStreamWriter::new(ChunkType::SDAT, self.inner, max_chunk_size),
            &context,
        )?;

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
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, EntryBuilder, WriteOptions};
    /// use std::fs::File;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let option = WriteOptions::builder().build();
    /// let file = File::create("example.pna")?;
    /// let mut archive = Archive::write_solid_header(file, option)?;
    /// archive
    ///     .add_entry(EntryBuilder::new_file("example.txt".into(), WriteOptions::store())?.build()?)?;
    /// archive.finalize()?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
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
    pub fn write_file<F>(&mut self, name: EntryName, metadata: Metadata, mut f: F) -> io::Result<()>
    where
        F: FnMut(&mut SolidArchiveEntryDataWriter<W>) -> io::Result<()>,
    {
        let option = WriteOptions::store();
        write_file_entry(
            &mut self.inner,
            name,
            metadata,
            option,
            self.max_chunk_size,
            |w| {
                let mut w = SolidArchiveEntryDataWriter(w);
                f(&mut w)?;
                Ok(w.0)
            },
        )
    }

    /// Sets the maximum chunk size for file data (FDAT) written via
    /// [`write_file()`](SolidArchive::write_file).
    ///
    /// This controls the inner FDAT chunk splitting for individual entries within
    /// the solid stream. The outer SDAT chunk size is fixed when `SolidArchive` is
    /// constructed and cannot be changed afterward. To control the outer SDAT chunk
    /// size, call [`Archive::set_max_chunk_size`] before
    /// [`into_solid_archive()`](Archive::into_solid_archive).
    ///
    /// Pre-built entries added via [`add_entry()`](SolidArchive::add_entry) use their own
    /// chunk size configured through [`EntryBuilder::max_chunk_size()`].
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
        (ChunkType::SEND, []).write_chunk_in(&mut inner)?;
        Ok(Archive::new(inner, self.archive_header))
    }
}

pub(crate) fn write_file_entry<W, F>(
    inner: &mut W,
    name: EntryName,
    metadata: Metadata,
    option: impl WriteOption,
    max_chunk_size: Option<NonZeroU32>,
    mut f: F,
) -> io::Result<()>
where
    W: Write,
    F: FnMut(InternalArchiveDataWriter<&mut W>) -> io::Result<InternalArchiveDataWriter<&mut W>>,
{
    let header = EntryHeader::for_file(
        option.compression(),
        option.encryption(),
        option.cipher_mode(),
        name,
    );
    (ChunkType::FHED, header.to_bytes()).write_chunk_in(inner)?;
    if let Some(c) = metadata.created {
        (ChunkType::cTIM, c.whole_seconds().to_be_bytes()).write_chunk_in(inner)?;
        if c.subsec_nanoseconds() != 0 {
            (ChunkType::cTNS, c.subsec_nanoseconds().to_be_bytes()).write_chunk_in(inner)?;
        }
    }
    if let Some(m) = metadata.modified {
        (ChunkType::mTIM, m.whole_seconds().to_be_bytes()).write_chunk_in(inner)?;
        if m.subsec_nanoseconds() != 0 {
            (ChunkType::mTNS, m.subsec_nanoseconds().to_be_bytes()).write_chunk_in(inner)?;
        }
    }
    if let Some(a) = metadata.accessed {
        (ChunkType::aTIM, a.whole_seconds().to_be_bytes()).write_chunk_in(inner)?;
        if a.subsec_nanoseconds() != 0 {
            (ChunkType::aTNS, a.subsec_nanoseconds().to_be_bytes()).write_chunk_in(inner)?;
        }
    }
    if let Some(p) = metadata.permission {
        (ChunkType::fPRM, p.to_bytes()).write_chunk_in(inner)?;
    }
    let context = get_writer_context(option)?;
    if let Some(WriteCipher { context: c, .. }) = &context.cipher {
        (ChunkType::PHSF, c.phsf.as_bytes()).write_chunk_in(inner)?;
        (ChunkType::FDAT, &c.iv[..]).write_chunk_in(inner)?;
    }
    let inner = {
        let writer = ChunkStreamWriter::new(ChunkType::FDAT, inner, max_chunk_size);
        let writer = get_writer(writer, &context)?;
        let mut writer = f(writer)?;
        writer.flush()?;
        writer.try_into_inner()?.try_into_inner()?.into_inner()
    };
    (ChunkType::FEND, Vec::<u8>::new()).write_chunk_in(inner)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReadOptions;
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
        let mut entries = reader.entries_with_password(None);
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
        let mut entries = reader.entries_with_password(None);
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

    fn count_chunks(archive: &[u8], ty: ChunkType) -> usize {
        crate::chunk::read_chunks_from_slice(archive)
            .unwrap()
            .filter(|c| c.as_ref().unwrap().ty() == ty)
            .count()
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
        let mut entries = reader.entries_with_password(None);
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
        let mut entries = reader.entries_with_password(None);
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
        let mut entries = reader.entries_with_password(None);
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
        let mut entries = reader.entries_with_password(None);
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
}
