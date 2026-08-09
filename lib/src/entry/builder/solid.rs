use super::prepend_data_prefix;
use crate::{
    archive::{InternalArchiveDataWriter, InternalDataWriter, write_stream_entry},
    chunk::{ChunkType, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{
        DataKind, Entry, EntryName, EntryWriteAttributes, NormalEntry, SolidEntry, SolidHeader,
        WriteCipher, WriteOption, WriteOptions, get_writer, get_writer_context,
        private::SealedEntryExt,
    },
    io::{FlattenWriter, TryIntoInner},
};
use std::{
    io::{self, prelude::*},
    num::NonZeroU32,
};

/// A writer for adding an entry payload within a [`SolidEntryBuilder`].
///
/// This struct provides a `Write` interface for adding content to an entry that
/// is being created within a solid entry. It is passed to the closure in
/// [`SolidEntryBuilder::write_file`] or [`SolidEntryBuilder::write_opaque`].
pub struct SolidEntryDataWriter<'a>(
    InternalArchiveDataWriter<&'a mut InternalDataWriter<FlattenWriter>>,
);

impl Write for SolidEntryDataWriter<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// A builder for creating a [`SolidEntry`].
///
/// This builder is used to construct a solid entry, which can contain multiple
/// files compressed together as a single unit. This is particularly effective
/// for achieving high compression ratios with many small, similar files.
///
/// # Examples
///
/// ```
/// # use std::io::{self, Write};
/// use libpna::{DirEntryBuilder, FileEntryBuilder, SolidEntryBuilder, WriteOptions};
///
/// # fn main() -> io::Result<()> {
/// let mut solid_builder = SolidEntryBuilder::new(WriteOptions::store())?;
///
/// // Add a directory to the solid entry
/// let dir_entry = DirEntryBuilder::new("my_dir/".into()).build()?;
/// solid_builder.add_entry(dir_entry)?;
///
/// // Add a file to the solid entry
/// let mut file_builder = FileEntryBuilder::new("my_dir/file.txt".into())?;
/// file_builder.write_all(b"This is a file inside a solid entry.")?;
/// solid_builder.add_entry(file_builder.build()?)?;
///
/// let solid_entry = solid_builder.build()?;
/// # Ok(())
/// # }
/// ```
pub struct SolidEntryBuilder {
    header: SolidHeader,
    phsf: Option<String>,
    prefix: Option<Vec<u8>>,
    max_chunk_size: NonZeroU32,
    data: CompressionWriter<CipherWriter<FlattenWriter>>,
    extra: Vec<RawChunk>,
    max_file_chunk_size: Option<NonZeroU32>,
}

impl SolidEntryBuilder {
    /// Creates a new [`SolidEntryBuilder`] with the given option.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new(option: impl WriteOption) -> io::Result<Self> {
        let header = SolidHeader::new(
            option.compression(),
            option.encryption(),
            option.cipher_mode(),
        );
        let context = get_writer_context(option, ChunkType::SHED, &header.to_bytes())?;
        let writer = get_writer(FlattenWriter::new(), &context)?;
        let (prefix, phsf) = match context.cipher {
            None => (None, None),
            Some(WriteCipher { context: c, .. }) => (Some(c.prefix_bytes()), Some(c.phsf)),
        };
        Ok(Self {
            header,
            prefix,
            max_chunk_size: NonZeroU32::MAX,
            phsf,
            data: writer,
            extra: Vec::new(),
            max_file_chunk_size: None,
        })
    }

    /// Adds an entry to the solid archive.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing a given entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{DirEntryBuilder, FileEntryBuilder, SolidEntryBuilder, WriteOptions};
    /// use std::io;
    /// use std::io::Write;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut builder = SolidEntryBuilder::new(WriteOptions::builder().build())?;
    /// let dir_entry = DirEntryBuilder::new("example".into()).build()?;
    /// builder.add_entry(dir_entry)?;
    /// let mut entry_builder = FileEntryBuilder::new("example/text.txt".into())?;
    /// entry_builder.write_all(b"content")?;
    /// let file_entry = entry_builder.build()?;
    /// builder.add_entry(file_entry)?;
    /// builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn add_entry<T>(&mut self, entry: NormalEntry<T>) -> io::Result<usize>
    where
        NormalEntry<T>: Entry,
    {
        entry.write_in(&mut self.data)
    }

    /// Writes a regular file to the solid entry.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry,
    /// or if the closure returns an error. If this method returns an error, the
    /// builder may contain a partial entry and must be discarded without
    /// further use.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::{Metadata, SolidEntryBuilder, WriteOptions};
    /// # use std::error::Error;
    /// use std::io::prelude::*;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let option = WriteOptions::builder().build();
    /// let mut builder = SolidEntryBuilder::new(option)?;
    /// builder.write_file("bar.txt".into(), Metadata::new(), |writer| {
    ///     writer.write_all(b"text")
    /// })?;
    /// builder.build()?;
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
        F: FnOnce(&mut SolidEntryDataWriter) -> io::Result<()>,
    {
        let option = WriteOptions::store();
        write_stream_entry(
            &mut self.data,
            name,
            DataKind::FILE,
            attributes.into(),
            option,
            self.max_file_chunk_size,
            |w| {
                let mut writer = SolidEntryDataWriter(w);
                f(&mut writer)?;
                Ok(writer.0)
            },
        )
    }

    /// Writes an opaque entry payload to the solid entry.
    ///
    /// The inner entry always uses STORE; compression and encryption are
    /// provided only by the outer solid stream. No validation is performed
    /// between `kind` and the payload. Prefer kind-specific APIs for data kinds
    /// defined by the PNA specification.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while writing the entry, or if
    /// the closure returns an error. If this method returns an error, the
    /// builder may contain a partial entry and must be discarded without
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
        F: FnOnce(&mut SolidEntryDataWriter) -> io::Result<()>,
    {
        write_stream_entry(
            &mut self.data,
            name,
            kind,
            attributes.into(),
            WriteOptions::store(),
            self.max_file_chunk_size,
            |w| {
                let mut writer = SolidEntryDataWriter(w);
                f(&mut writer)?;
                Ok(writer.0)
            },
        )
    }

    /// Adds extra chunk to the solid entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) {
        self.extra.push(chunk.into());
    }

    /// Sets the maximum chunk size for data written to this solid entry.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::io::{self, Write};
    /// use std::num::NonZeroU32;
    /// use libpna::{FileEntryBuilder, SolidEntryBuilder, WriteOptions};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut solid_builder = SolidEntryBuilder::new(WriteOptions::store())?;
    /// solid_builder.max_chunk_size(NonZeroU32::new(1024 * 1024).unwrap()); // 1MB chunks
    ///
    /// let file_entry = FileEntryBuilder::new("file.txt".into())?;
    /// solid_builder.add_entry(file_entry.build()?)?;
    ///
    /// let solid_entry = solid_builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn max_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        self.max_chunk_size = size;
        self.data
            .get_mut()
            .get_mut()
            .set_max_chunk_size(size.get() as usize);
        self
    }

    /// Sets the maximum chunk size for entry data (FDAT) written via
    /// [`write_file()`](SolidEntryBuilder::write_file) or
    /// [`write_opaque()`](SolidEntryBuilder::write_opaque).
    ///
    /// This is independent of [`max_chunk_size()`](SolidEntryBuilder::max_chunk_size),
    /// which controls the outer data chunking.
    ///
    /// The default is the maximum allowed chunk size (~4GB).
    #[inline]
    pub fn max_file_chunk_size(&mut self, size: NonZeroU32) -> &mut Self {
        self.max_file_chunk_size = Some(size);
        self
    }

    fn build_as_entry(self) -> io::Result<SolidEntry> {
        Ok(SolidEntry {
            header: self.header,
            phsf: self.phsf,
            data: {
                let mut data = self.data.try_into_inner()?.try_into_inner()?.inner;
                if let Some(prefix) = self.prefix {
                    prepend_data_prefix(&mut data, prefix, self.max_chunk_size);
                }
                data
            },
            extra: self.extra,
        })
    }

    /// Consumes this builder and returns the constructed [`Entry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry into buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{SolidEntryBuilder, WriteOptions};
    /// use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let builder = SolidEntryBuilder::new(WriteOptions::builder().build())?;
    /// let entries = builder.build()?;
    /// #     Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<SolidEntry> {
        self.build_as_entry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkType, CipherMode, Compression, Encryption, HashAlgorithm, Metadata, ReadOptions,
    };
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn solid_entry_extra_chunk() {
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder.add_extra_chunk(RawChunk::from_data(
            ChunkType::private(*b"abCd").unwrap(),
            [],
        ));
        let entry = builder.build_as_entry().unwrap();

        assert_eq!(
            &entry.extra[0],
            &RawChunk::from_data(ChunkType::private(*b"abCd").unwrap(), []),
        );
    }

    #[test]
    fn solid_entry_builder_write_file_with_max_chunk_size() {
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder.max_chunk_size(NonZeroU32::new(8).unwrap());
        builder
            .write_file("entry".into(), Metadata::new(), |w| {
                w.write_all(b"abcdefghijklmnopqrstuvwxyz")
            })
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();
        let mut entries = solid_entry.entries(ReadOptions::builder().build()).unwrap();
        let entry = entries.next().unwrap().unwrap();
        let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(b"abcdefghijklmnopqrstuvwxyz", &buf[..]);
        assert!(
            solid_entry.data.len() > 1,
            "Data should be split into multiple chunks"
        );
    }

    #[test]
    fn solid_entry_gcm_header_respects_max_chunk_size() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::GCM)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .build();
        let mut builder = SolidEntryBuilder::new(options).unwrap();
        builder.max_chunk_size(NonZeroU32::new(8).unwrap());
        builder
            .write_file("entry".into(), Metadata::new(), |w| w.write_all(b"x"))
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();

        assert!(
            solid_entry.data.iter().all(|chunk| chunk.len() <= 8),
            "every SDAT body must respect max_chunk_size"
        );

        let mut entries = solid_entry
            .entries(ReadOptions::with_password(Some("password")))
            .unwrap();
        let entry = entries.next().unwrap().unwrap();
        let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
        let mut plain = Vec::new();
        reader.read_to_end(&mut plain).unwrap();
        assert_eq!(plain, b"x");
    }

    #[test]
    fn solid_entry_builder_write_file() {
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder
            .write_file("entry".into(), Metadata::new(), |w| {
                w.write_all("テストデータ".as_bytes())
            })
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();

        let mut entries = solid_entry.entries(ReadOptions::builder().build()).unwrap();
        let entry = entries.next().unwrap().unwrap();
        let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();

        assert_eq!("テストデータ".as_bytes(), &buf[..]);
    }

    #[test]
    fn solid_entry_builder_write_opaque_uses_store() {
        let kind = DataKind::new_private(202).unwrap();
        let mut builder = SolidEntryBuilder::new(
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .build(),
        )
        .unwrap();
        builder
            .write_opaque("entry".into(), kind, Metadata::new(), |writer| {
                writer.write_all(b"opaque")
            })
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();

        let mut entries = solid_entry.entries(ReadOptions::builder().build()).unwrap();
        let entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.header().data_kind(), kind);
        assert_eq!(entry.header().compression(), Compression::NO);
        let mut data = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"opaque");
    }

    #[test]
    fn solid_write_file_with_xattrs_metadata_round_trips() {
        use crate::entry::{ExtendedAttribute, XattrName, XattrValue};
        let xattr = ExtendedAttribute::new(
            XattrName::try_from("user.k").unwrap(),
            XattrValue::try_from(b"v".as_slice()).unwrap(),
        );
        let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
        builder
            .write_file(
                "entry".into(),
                Metadata::new().with_xattrs(vec![xattr.clone()]),
                |w| w.write_all(b"data"),
            )
            .unwrap();
        let solid_entry = builder.build_as_entry().unwrap();
        let mut entries = solid_entry.entries(ReadOptions::builder().build()).unwrap();
        let entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.metadata().xattrs(), &[xattr]);
    }
}
