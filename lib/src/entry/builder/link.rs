//! Builders for symbolic link and hard link entries.
use super::{EntryBuilderCore, data_writer};
use crate::{
    CipherMode, Encryption, Metadata, NormalEntry, WriteOptions,
    chunk::RawChunk,
    cipher::CipherWriter,
    compress::CompressionWriter,
    entry::{EntryHeader, EntryName, EntryReference, WriteOption},
    io::{FlattenWriter, TryIntoInner},
};
use std::io::{self, Write};

struct LinkEntryBuilder {
    core: EntryBuilderCore,
    data: CompressionWriter<CipherWriter<FlattenWriter>>,
}

impl LinkEntryBuilder {
    fn new(
        header: EntryHeader,
        source: EntryReference,
        option: impl WriteOption,
    ) -> io::Result<Self> {
        let (mut writer, iv, phsf) = data_writer(option)?;
        writer.write_all(source.as_bytes())?;
        let mut core = EntryBuilderCore::new(header);
        core.set_cipher(iv, phsf);
        Ok(Self { core, data: writer })
    }

    fn build(self) -> io::Result<NormalEntry> {
        let data = self.data.try_into_inner()?.try_into_inner()?.inner;
        Ok(self.core.build(data, None))
    }
}

/// A builder for creating a symbolic link [`NormalEntry`].
///
/// The entry data is the UTF-8 encoded link target path, fixed at
/// construction time; this builder does not implement
/// [`Write`](std::io::Write).
pub struct SymlinkEntryBuilder(LinkEntryBuilder);

impl SymlinkEntryBuilder {
    /// Creates a builder that stores the link target without
    /// compression or encryption.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_with_options(name, source, WriteOptions::store())
    }

    /// Creates a builder with the given write options; the link
    /// target is compressed and encrypted accordingly.
    ///
    /// When `option.encryption()` is [`Encryption::NO`], the cipher
    /// mode recorded in the header is [`CipherMode::CBC`] regardless
    /// of `option.cipher_mode()`, matching the wire representation
    /// produced by the legacy link constructors.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new_with_options(
        name: EntryName,
        source: EntryReference,
        option: impl WriteOption,
    ) -> io::Result<Self> {
        let encryption = option.encryption();
        let cipher_mode = if encryption == Encryption::NO {
            CipherMode::CBC
        } else {
            option.cipher_mode()
        };
        let header = EntryHeader::new_with_options(
            crate::DataKind::SYMBOLIC_LINK,
            option.compression(),
            encryption,
            cipher_mode,
            name,
        );
        Ok(Self(LinkEntryBuilder::new(header, source, option)?))
    }

    /// Sets the metadata of the entry, replacing any previously set
    /// metadata.
    ///
    /// The raw file size and compressed size recorded in the given
    /// metadata are ignored; [`build()`](Self::build) computes them.
    #[inline]
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.0.core.metadata(metadata);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.0.core.add_extra_chunk(chunk);
        self
    }

    /// Consumes this builder and returns the constructed
    /// [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry
    /// into buffer.
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<NormalEntry> {
        self.0.build()
    }
}

/// A builder for creating a hard link [`NormalEntry`].
///
/// The entry data is the UTF-8 encoded path of the target entry within
/// the same archive, fixed at construction time; this builder does not
/// implement [`Write`](std::io::Write).
pub struct HardLinkEntryBuilder(LinkEntryBuilder);

impl HardLinkEntryBuilder {
    /// Creates a builder that stores the link target without
    /// compression or encryption.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new(name: EntryName, source: EntryReference) -> io::Result<Self> {
        Self::new_with_options(name, source, WriteOptions::store())
    }

    /// Creates a builder with the given write options; the link
    /// target is compressed and encrypted accordingly.
    ///
    /// When `option.encryption()` is [`Encryption::NO`], the cipher
    /// mode recorded in the header is [`CipherMode::CBC`] regardless
    /// of `option.cipher_mode()`, matching the wire representation
    /// produced by the legacy link constructors.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[inline]
    pub fn new_with_options(
        name: EntryName,
        source: EntryReference,
        option: impl WriteOption,
    ) -> io::Result<Self> {
        let encryption = option.encryption();
        let cipher_mode = if encryption == Encryption::NO {
            CipherMode::CBC
        } else {
            option.cipher_mode()
        };
        let header = EntryHeader::new_with_options(
            crate::DataKind::HARD_LINK,
            option.compression(),
            encryption,
            cipher_mode,
            name,
        );
        Ok(Self(LinkEntryBuilder::new(header, source, option)?))
    }

    /// Sets the metadata of the entry, replacing any previously set
    /// metadata.
    ///
    /// The raw file size and compressed size recorded in the given
    /// metadata are ignored; [`build()`](Self::build) computes them.
    #[inline]
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.0.core.metadata(metadata);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.0.core.add_extra_chunk(chunk);
        self
    }

    /// Consumes this builder and returns the constructed
    /// [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry
    /// into buffer.
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<NormalEntry> {
        self.0.build()
    }
}
