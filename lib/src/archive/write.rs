use crate::{
    archive::{Archive, ArchiveHeader, SolidArchive, PNA_HEADER},
    chunk::{Chunk, ChunkExt, ChunkStreamWriter, ChunkType, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        get_writer, get_writer_context, Entry, EntryHeader, EntryName, EntryPart, Metadata,
        NormalEntry, SealedEntryExt, SolidHeader, WriteCipher, WriteOption, WriteOptions,
    },
    io::TryIntoInner,
};
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
        write.write_all(PNA_HEADER)?;
        (ChunkType::AHED, header.to_bytes()).write_chunk_in(&mut write)?;
        Ok(Self::new(write, header))
    }

    /// Write a regular file as normal entry into archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing an entry, or if the given closure returns an error return it.
    ///
    /// # Example
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
        metadata: Metadata,
        option: impl WriteOption,
        mut f: F,
    ) -> io::Result<()>
    where
        F: FnMut(&mut EntryDataWriter<&mut W>) -> io::Result<()>,
    {
        write_file_entry(&mut self.inner, name, metadata, option, |w| {
            let mut w = EntryDataWriter(w);
            f(&mut w)?;
            Ok(w.0)
        })
    }

    /// Adds a new entry to the archive.
    ///
    /// # Arguments
    ///
    /// * `entry` - The entry to add to the archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
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
        self.add_next_archive_marker()?;
        self.finalize()?;
        Archive::write_header_with(writer, header)
    }

    /// Write an end marker to finalize the archive.
    ///
    /// Marks that the PNA archive contains no more entries.
    /// Normally, a PNA archive reader will continue reading entries in the hope that the entry exists until it encounters this end marker.
    /// This end marker should always be recorded at the end of the file unless there is a special reason to do so.
    ///
    /// # Errors
    /// Returns an error if failed to write archive end marker.
    ///
    /// # Examples
    /// Create an empty archive.
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

    /// Write an end marker to finalize the archive.
    /// This API is unstable.
    ///
    /// # Errors
    ///
    /// Returns an error if failed to write archive end marker.
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
        let context = get_writer_context(option)?;

        (ChunkType::SHED, header.to_bytes()).write_chunk_in(&mut self.inner)?;
        if let Some(WriteCipher { context: c, .. }) = &context.cipher {
            (ChunkType::PHSF, c.phsf.as_bytes()).write_chunk_in(&mut self.inner)?;
            (ChunkType::SDAT, c.iv.as_slice()).write_chunk_in(&mut self.inner)?;
        }
        self.inner.flush()?;
        let writer = get_writer(
            ChunkStreamWriter::new(ChunkType::SDAT, self.inner),
            &context,
        )?;

        Ok(SolidArchive {
            archive_header: self.header,
            inner: writer,
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
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
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
    #[inline]
    pub fn add_entry<T>(&mut self, entry: NormalEntry<T>) -> io::Result<usize>
    where
        NormalEntry<T>: Entry,
    {
        entry.write_in(&mut self.inner)
    }

    /// Write a regular file as solid entry into archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing an entry, or if the given closure returns an error return it.
    ///
    /// # Example
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
        write_file_entry(&mut self.inner, name, metadata, option, |w| {
            let mut w = SolidArchiveEntryDataWriter(w);
            f(&mut w)?;
            Ok(w.0)
        })
    }

    /// Write an end marker to finalize the archive.
    ///
    /// Marks that the PNA archive contains no more entries.
    /// Normally, a PNA archive reader will continue reading entries in the hope that the entry exists until it encounters this end marker.
    /// This end marker should always be recorded at the end of the file unless there is a special reason to do so.
    ///
    /// # Errors
    /// Returns an error if failed to write archive end marker.
    ///
    /// # Examples
    /// Create an empty archive.
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

fn write_file_entry<W, F>(
    inner: &mut W,
    name: EntryName,
    metadata: Metadata,
    option: impl WriteOption,
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
        (ChunkType::cTIM, c.as_secs().to_be_bytes()).write_chunk_in(inner)?;
    }
    if let Some(m) = metadata.modified {
        (ChunkType::mTIM, m.as_secs().to_be_bytes()).write_chunk_in(inner)?;
    }
    if let Some(a) = metadata.accessed {
        (ChunkType::aTIM, a.as_secs().to_be_bytes()).write_chunk_in(inner)?;
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
        let writer = ChunkStreamWriter::new(ChunkType::FDAT, inner);
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

    #[cfg(feature = "unstable-async")]
    #[tokio::test]
    async fn encode_async() {
        use futures_util::AsyncReadExt;
        use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

        let archive_bytes = {
            let file = Vec::new().compat_write();
            let writer = Archive::write_header_async(file).await.unwrap();
            writer.finalize_async().await.unwrap().into_inner()
        };
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(archive_bytes.as_slice(), expected.as_slice());
    }
}
