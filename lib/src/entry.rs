//! Types for representing entries in a PNA archive.

mod attr;
mod builder;
mod content;
mod header;
mod meta;
mod name;
mod options;
mod read;
mod reference;
mod write;

#[allow(deprecated)]
pub use self::{
    attr::*,
    builder::{
        DirEntryBuilder, EntryBuilder, FileEntryBuilder, HardLinkEntryBuilder, OpaqueEntryBuilder,
        SolidEntryBuilder, SymlinkEntryBuilder,
    },
    content::*,
    header::*,
    meta::*,
    name::*,
    options::*,
    reference::*,
};
pub(crate) use self::{private::*, read::*, write::*};
use crate::{
    Duration,
    chunk::{Chunk, ChunkExt, ChunkType, MIN_CHUNK_BYTES_SIZE, RawChunk, chunk_data_split},
    ext::time::DurationExt,
    util::io::ChainReader,
    util::slice::skip_while,
};
use std::{
    borrow::Cow,
    collections::VecDeque,
    convert::Infallible,
    io::{self, Read, Write},
};

mod private {
    use super::*;

    pub trait EntryChunkSink {
        type Error;

        fn write_chunk<C>(&mut self, chunk: C) -> Result<(), Self::Error>
        where
            C: Chunk + Into<RawChunk>;
    }

    pub trait SealedEntryExt {
        /// Emits every chunk of this entry in wire order.
        fn write_chunks_to<S: EntryChunkSink>(self, sink: &mut S) -> Result<(), S::Error>;

        fn into_chunks(self) -> Vec<RawChunk>
        where
            Self: Sized,
        {
            let mut sink = RawChunkCollector(Vec::new());
            self.write_chunks_to(&mut sink)
                .expect("raw chunk collection is infallible");
            sink.0
        }

        fn write_in<W: Write>(self, writer: &mut W) -> io::Result<usize>
        where
            Self: Sized,
        {
            let mut sink = EntryChunkWriter {
                writer,
                written_len: 0,
            };
            self.write_chunks_to(&mut sink)?;
            Ok(sink.written_len)
        }
    }
}

/// A trait representing an entry in a PNA archive.
pub trait Entry: SealedEntryExt {}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct RawEntry<T = Vec<u8>>(pub(crate) Vec<RawChunk<T>>);

struct RawChunkCollector(Vec<RawChunk>);

impl EntryChunkSink for RawChunkCollector {
    type Error = Infallible;

    #[inline]
    fn write_chunk<C>(&mut self, chunk: C) -> Result<(), Self::Error>
    where
        C: Chunk + Into<RawChunk>,
    {
        self.0.push(chunk.into());
        Ok(())
    }
}

struct EntryChunkWriter<'w, W> {
    writer: &'w mut W,
    written_len: usize,
}

impl<W: Write> EntryChunkSink for EntryChunkWriter<'_, W> {
    type Error = io::Error;

    #[inline]
    fn write_chunk<C>(&mut self, chunk: C) -> Result<(), Self::Error>
    where
        C: Chunk + Into<RawChunk>,
    {
        self.written_len += crate::io::write_chunk(self.writer, chunk)?;
        Ok(())
    }
}

#[allow(deprecated)]
fn try_for_each_metadata_facet<E>(
    metadata: &Metadata,
    mut f: impl FnMut(ChunkType, Cow<[u8]>) -> Result<(), E>,
) -> Result<(), E> {
    if let Some(value) = metadata.created {
        let (seconds, nanos) = value.to_seconds_nanos();
        f(ChunkType::cTIM, Cow::Borrowed(&seconds.to_be_bytes()))?;
        if nanos != 0 {
            f(ChunkType::cTNS, Cow::Borrowed(&nanos.to_be_bytes()))?;
        }
    }
    if let Some(value) = metadata.modified {
        let (seconds, nanos) = value.to_seconds_nanos();
        f(ChunkType::mTIM, Cow::Borrowed(&seconds.to_be_bytes()))?;
        if nanos != 0 {
            f(ChunkType::mTNS, Cow::Borrowed(&nanos.to_be_bytes()))?;
        }
    }
    if let Some(value) = metadata.accessed {
        let (seconds, nanos) = value.to_seconds_nanos();
        f(ChunkType::aTIM, Cow::Borrowed(&seconds.to_be_bytes()))?;
        if nanos != 0 {
            f(ChunkType::aTNS, Cow::Borrowed(&nanos.to_be_bytes()))?;
        }
    }
    if let Some(value) = &metadata.permission {
        f(ChunkType::fPRM, Cow::Owned(value.to_bytes()))?;
    }
    if let Some(value) = metadata.owner_uid {
        f(ChunkType::fUId, Cow::Borrowed(&value.to_bytes()))?;
    }
    if let Some(value) = metadata.owner_gid {
        f(ChunkType::fGId, Cow::Borrowed(&value.to_bytes()))?;
    }
    if let Some(value) = &metadata.owner_user_name {
        f(ChunkType::fONm, Cow::Owned(value.to_bytes()))?;
    }
    if let Some(value) = &metadata.owner_group_name {
        f(ChunkType::fGNm, Cow::Owned(value.to_bytes()))?;
    }
    if let Some(value) = &metadata.owner_user_sid {
        f(ChunkType::fOSi, Cow::Owned(value.to_bytes()))?;
    }
    if let Some(value) = &metadata.owner_group_sid {
        f(ChunkType::fGSi, Cow::Owned(value.to_bytes()))?;
    }
    if let Some(value) = metadata.permission_mode {
        f(ChunkType::fMOd, Cow::Borrowed(&value.to_bytes()))?;
    }
    if let Some(value) = metadata.link_target_type {
        f(ChunkType::fLTP, Cow::Borrowed(&value.to_bytes()))?;
    }
    for value in &metadata.xattrs {
        f(ChunkType::xATR, Cow::Owned(value.to_bytes()))?;
    }
    Ok(())
}

pub(crate) fn write_metadata_facets<W: Write>(
    writer: &mut W,
    metadata: &Metadata,
) -> io::Result<usize> {
    let mut total = 0;
    try_for_each_metadata_facet(metadata, |ty, data| {
        total += crate::io::write_chunk(writer, (ty, data))?;
        Ok::<(), io::Error>(())
    })?;
    Ok(total)
}

impl<T> SealedEntryExt for RawEntry<T>
where
    RawChunk<T>: Chunk + Into<RawChunk>,
{
    #[inline]
    fn write_chunks_to<S: EntryChunkSink>(self, sink: &mut S) -> Result<(), S::Error> {
        for chunk in self.0 {
            sink.write_chunk(chunk)?;
        }
        Ok(())
    }
}

impl<T> Entry for RawEntry<T> where RawEntry<T>: SealedEntryExt {}

impl<'a> From<RawEntry<Cow<'a, [u8]>>> for RawEntry<Vec<u8>> {
    #[inline]
    fn from(value: RawEntry<Cow<'a, [u8]>>) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl<'a> From<RawEntry<&'a [u8]>> for RawEntry<Vec<u8>> {
    #[inline]
    fn from(value: RawEntry<&'a [u8]>) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl From<RawEntry<Vec<u8>>> for RawEntry<Cow<'_, [u8]>> {
    #[inline]
    fn from(value: RawEntry<Vec<u8>>) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl<'a> From<RawEntry<&'a [u8]>> for RawEntry<Cow<'a, [u8]>> {
    #[inline]
    fn from(value: RawEntry<&'a [u8]>) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

/// Reader for Entry data.
pub struct EntryDataReader<'r>(EntryReader<EncodedDataReader<'r>>);

impl<'r> Read for EntryDataReader<'r> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

#[cfg(feature = "unstable-async")]
impl<'r> futures_io::AsyncRead for EntryDataReader<'r> {
    #[inline]
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::task::Poll::Ready(self.get_mut().read(buf))
    }
}

/// Reader for encoded entry data.
///
/// This reader returns the concatenated body bytes of `FDAT` chunks for
/// [`NormalEntry`] and `SDAT` chunks for [`SolidEntry`]. The returned stream is
/// the data as stored in the archive, before decryption or decompression, and
/// does not include chunk length, type, or CRC bytes.
pub struct EncodedDataReader<'r>(ChainReader<std::vec::IntoIter<&'r [u8]>, &'r [u8]>);

impl<'r> Read for EncodedDataReader<'r> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

#[cfg(feature = "unstable-async")]
impl<'r> futures_io::AsyncRead for EncodedDataReader<'r> {
    #[inline]
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::task::Poll::Ready(self.get_mut().read(buf))
    }
}

#[inline]
fn encoded_data_reader<T: AsRef<[u8]>>(data: &[T]) -> EncodedDataReader<'_> {
    EncodedDataReader(ChainReader::new(
        data.iter()
            .map(AsRef::as_ref as fn(&T) -> &[u8])
            .collect::<Vec<_>>(),
    ))
}

/// A [`NormalEntry`] or [`SolidEntry`] read from an archive.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ReadEntry<T = Vec<u8>> {
    /// A solid mode entry that contains multiple files compressed together.
    /// This type of entry provides better compression ratios but requires
    /// sequential access to the contained files.
    Solid(SolidEntry<T>),
    /// A normal entry that represents a single file in the archive.
    /// This type of entry allows random access to the file data.
    Normal(NormalEntry<T>),
}

impl<T> SealedEntryExt for ReadEntry<T>
where
    NormalEntry<T>: SealedEntryExt,
    SolidEntry<T>: SealedEntryExt,
{
    #[inline]
    fn write_chunks_to<S: EntryChunkSink>(self, sink: &mut S) -> Result<(), S::Error> {
        match self {
            Self::Normal(r) => r.write_chunks_to(sink),
            Self::Solid(s) => s.write_chunks_to(sink),
        }
    }
}

impl<T> Entry for ReadEntry<T> where ReadEntry<T>: SealedEntryExt {}

impl<T> TryFrom<RawEntry<T>> for ReadEntry<T>
where
    RawChunk<T>: Chunk,
{
    type Error = io::Error;

    #[inline]
    fn try_from(entry: RawEntry<T>) -> Result<Self, Self::Error> {
        if let Some(first_chunk) = entry.0.first() {
            match first_chunk.ty {
                ChunkType::SHED => Ok(Self::Solid(SolidEntry::try_from(entry)?)),
                ChunkType::FHED => Ok(Self::Normal(NormalEntry::try_from(entry)?)),
                _ => Err(io::Error::new(io::ErrorKind::InvalidData, "invalid entry")),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "empty entry"))
        }
    }
}

impl<T> From<NormalEntry<T>> for ReadEntry<T> {
    #[inline]
    fn from(value: NormalEntry<T>) -> Self {
        Self::Normal(value)
    }
}

impl<T> From<SolidEntry<T>> for ReadEntry<T> {
    #[inline]
    fn from(value: SolidEntry<T>) -> Self {
        Self::Solid(value)
    }
}

impl<'a> From<ReadEntry<Cow<'a, [u8]>>> for ReadEntry<Vec<u8>> {
    #[inline]
    fn from(value: ReadEntry<Cow<'a, [u8]>>) -> Self {
        match value {
            ReadEntry::Solid(s) => Self::Solid(s.into()),
            ReadEntry::Normal(r) => Self::Normal(r.into()),
        }
    }
}

impl<'a> From<ReadEntry<&'a [u8]>> for ReadEntry<Vec<u8>> {
    #[inline]
    fn from(value: ReadEntry<&'a [u8]>) -> Self {
        match value {
            ReadEntry::Solid(s) => Self::Solid(s.into()),
            ReadEntry::Normal(r) => Self::Normal(r.into()),
        }
    }
}

impl From<ReadEntry<Vec<u8>>> for ReadEntry<Cow<'_, [u8]>> {
    #[inline]
    fn from(value: ReadEntry<Vec<u8>>) -> Self {
        match value {
            ReadEntry::Solid(s) => Self::Solid(s.into()),
            ReadEntry::Normal(r) => Self::Normal(r.into()),
        }
    }
}

impl<'a> From<ReadEntry<&'a [u8]>> for ReadEntry<Cow<'a, [u8]>> {
    #[inline]
    fn from(value: ReadEntry<&'a [u8]>) -> Self {
        match value {
            ReadEntry::Solid(s) => Self::Solid(s.into()),
            ReadEntry::Normal(r) => Self::Normal(r.into()),
        }
    }
}

pub(crate) struct EntryIterator<'s>(EntryReader<EncodedDataReader<'s>>);

#[inline]
fn read_next_normal_entry_from_stream<R: Read>(reader: &mut R) -> Option<io::Result<NormalEntry>> {
    let mut chunks = Vec::new();
    loop {
        let chunk = crate::io::read_chunk(reader, u32::MAX);
        match chunk {
            Ok(chunk) => match chunk.ty {
                ChunkType::FEND => {
                    chunks.push(chunk);
                    break;
                }
                _ => chunks.push(chunk),
            },
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return if chunks.is_empty() {
                    None
                } else {
                    Some(Err(e))
                };
            }
            Err(e) => return Some(Err(e)),
        }
    }
    Some(RawEntry(chunks).try_into())
}

impl Iterator for EntryIterator<'_> {
    type Item = io::Result<NormalEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        read_next_normal_entry_from_stream(&mut self.0)
    }
}

type BytesCursor = io::Cursor<Vec<u8>>;

/// An iterator that moves out of a solid entry.
///
/// This struct is created by the `into_entries` method on [`SolidEntry`].
pub(crate) struct SolidIntoEntries(
    EntryReader<ChainReader<std::vec::IntoIter<BytesCursor>, BytesCursor>>,
);

impl Iterator for SolidIntoEntries {
    type Item = io::Result<NormalEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        read_next_normal_entry_from_stream(&mut self.0)
    }
}

/// A solid mode entry in a PNA archive.
///
/// Solid entries contain multiple files compressed together as a single unit.
/// This provides better compression ratios but requires sequential access to
/// the contained files. The entry includes a header, optional password hash,
/// data chunks, and any extra chunks.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SolidEntry<T = Vec<u8>> {
    header: SolidHeader,
    phsf: Option<String>,
    data: Vec<T>,
    extra: Vec<RawChunk<T>>,
}

impl<T> SealedEntryExt for SolidEntry<T>
where
    RawChunk<T>: Chunk + Into<RawChunk>,
    (ChunkType, T): Chunk + Into<RawChunk>,
{
    #[inline]
    fn write_chunks_to<S: EntryChunkSink>(self, sink: &mut S) -> Result<(), S::Error> {
        sink.write_chunk((ChunkType::SHED, self.header.to_bytes()))?;
        for extra_chunk in self.extra {
            sink.write_chunk(extra_chunk)?;
        }
        if let Some(phsf) = self.phsf {
            sink.write_chunk((ChunkType::PHSF, phsf.into_bytes()))?;
        }
        for data in self.data {
            sink.write_chunk((ChunkType::SDAT, data))?;
        }
        sink.write_chunk((ChunkType::SEND, []))
    }
}

impl<T> Entry for SolidEntry<T> where SolidEntry<T>: SealedEntryExt {}

impl<T> SolidEntry<T> {
    /// Returns the header of this solid entry.
    #[inline]
    pub fn header(&self) -> &SolidHeader {
        &self.header
    }

    /// Returns the compression method of this solid entry.
    #[inline]
    pub const fn compression(&self) -> Compression {
        self.header.compression
    }

    /// Returns the encryption method of this solid entry.
    #[inline]
    pub const fn encryption(&self) -> Encryption {
        self.header.encryption
    }

    /// Returns the cipher mode of this solid entry's encryption method.
    #[inline]
    pub const fn cipher_mode(&self) -> CipherMode {
        self.header.cipher_mode
    }

    /// Returns the extra chunks of this solid entry.
    #[inline]
    pub fn extra_chunks(&self) -> &[RawChunk<T>] {
        &self.extra
    }
}

impl<T: AsRef<[u8]>> SolidEntry<T> {
    /// Returns a reader over the encoded `SDAT` chunk body bytes.
    ///
    /// This reader exposes the solid entry data as stored in the archive,
    /// before decryption or decompression. It returns the concatenated bodies
    /// of all `SDAT` chunks and does not include chunk length, type, or CRC
    /// bytes.
    #[inline]
    pub fn encoded_reader(&self) -> EncodedDataReader<'_> {
        encoded_data_reader(&self.data)
    }

    /// Returns an iterator over the entries in the [`SolidEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the [`SolidEntry`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, ReadEntry, ReadOptions};
    /// use std::fs;
    /// # use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// for entry in archive.entries() {
    ///     match entry? {
    ///         ReadEntry::Solid(solid_entry) => {
    ///             let options = ReadOptions::with_password(Some(b"password"));
    ///             for entry in solid_entry.entries(&options)? {
    ///                 let entry = entry?;
    ///                 let mut reader = entry.reader(ReadOptions::builder().build());
    ///                 // process the entry
    ///             }
    ///         }
    ///         ReadEntry::Normal(_entry) => {
    ///             // process the entry
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn entries<'a, O: ReadOption>(
        &'a self,
        options: O,
    ) -> io::Result<impl Iterator<Item = io::Result<NormalEntry>> + use<'a, T, O>> {
        let reader = decrypt_reader(
            self.encoded_reader(),
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            &options,
            ChunkType::SHED,
            &self.header.to_bytes(),
        )?;
        let reader = decompress_reader(reader, self.header.compression)?;

        Ok(EntryIterator(EntryReader(reader)))
    }
}

impl<T> SolidEntry<T>
where
    T: Into<Vec<u8>>,
{
    /// Consumes this solid entry and returns a streaming iterator of normal
    /// entries, reusing derived keys cached in the supplied options.
    #[inline]
    pub(crate) fn into_entries_with_options(
        self,
        options: &ReadOptions,
    ) -> io::Result<SolidIntoEntries> {
        let bufs = self
            .data
            .into_iter()
            .map(|v| io::Cursor::new(v.into()))
            .collect::<Vec<_>>();
        let chain = ChainReader::new(bufs);
        let reader = decrypt_reader(
            chain,
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            options,
            ChunkType::SHED,
            &self.header.to_bytes(),
        )?;
        let reader = decompress_reader(reader, self.header.compression)?;
        Ok(SolidIntoEntries(EntryReader(reader)))
    }
}

impl<'a> From<SolidEntry<Cow<'a, [u8]>>> for SolidEntry<Vec<u8>> {
    #[inline]
    fn from(value: SolidEntry<Cow<'a, [u8]>>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            data: value.data.into_iter().map(Into::into).collect(),
            extra: value.extra.into_iter().map(Into::into).collect(),
        }
    }
}

impl<'a> From<SolidEntry<&'a [u8]>> for SolidEntry<Vec<u8>> {
    #[inline]
    fn from(value: SolidEntry<&'a [u8]>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            data: value.data.into_iter().map(Into::into).collect(),
            extra: value.extra.into_iter().map(Into::into).collect(),
        }
    }
}

impl<'a> From<SolidEntry<&'a [u8]>> for SolidEntry<Cow<'a, [u8]>> {
    #[inline]
    fn from(value: SolidEntry<&'a [u8]>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            data: value.data.into_iter().map(Into::into).collect(),
            extra: value.extra.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SolidEntry<Vec<u8>>> for SolidEntry<Cow<'_, [u8]>> {
    #[inline]
    fn from(value: SolidEntry<Vec<u8>>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            data: value.data.into_iter().map(Into::into).collect(),
            extra: value.extra.into_iter().map(Into::into).collect(),
        }
    }
}

impl<T> TryFrom<RawEntry<T>> for SolidEntry<T>
where
    RawChunk<T>: Chunk,
{
    type Error = io::Error;

    #[inline]
    fn try_from(entry: RawEntry<T>) -> Result<Self, Self::Error> {
        let mut chunks = entry.0.into_iter();
        let header = if let Some(first_chunk) = chunks.next() {
            if first_chunk.ty != ChunkType::SHED {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "expected `{}` chunk, got `{}`",
                        ChunkType::SHED,
                        first_chunk.ty
                    ),
                ));
            } else {
                SolidHeader::try_from(first_chunk.data())?
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("`{}` chunk not found", ChunkType::SHED),
            ));
        };
        if header.major != 0 || header.minor != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "solid entry version {}.{} is not supported",
                    header.major, header.minor
                ),
            ));
        }
        let mut extra = vec![];
        let mut data = vec![];
        let mut phsf = None;
        for chunk in chunks {
            match chunk.ty() {
                ChunkType::SEND => break,
                ChunkType::SDAT => data.push(chunk.data),
                ChunkType::PHSF => {
                    phsf = Some(
                        String::from_utf8(chunk.data().into())
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    )
                }
                _ => {
                    if chunk.ty().is_critical() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unknown critical chunk type: `{}`", chunk.ty()),
                        ));
                    }
                    extra.push(chunk);
                }
            }
        }
        Ok(Self {
            header,
            phsf,
            data,
            extra,
        })
    }
}

/// A normal entry in a PNA archive.
///
/// Normal entries represent individual files in the archive, allowing for
/// random access to the file data. Each entry includes a header, optional
/// password hash, data chunks, metadata, extended attributes, and any extra chunks.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct NormalEntry<T = Vec<u8>> {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<RawChunk<T>>,
    pub(crate) data: Vec<T>,
    pub(crate) metadata: Metadata,
}

impl<T> TryFrom<RawEntry<T>> for NormalEntry<T>
where
    RawChunk<T>: Chunk,
{
    type Error = io::Error;

    #[allow(deprecated)]
    #[inline]
    fn try_from(entry: RawEntry<T>) -> Result<Self, Self::Error> {
        let mut chunks = entry.0.into_iter();
        let header = if let Some(first_chunk) = chunks.next() {
            if first_chunk.ty != ChunkType::FHED {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "expected `{}` chunk, got `{}`",
                        ChunkType::FHED,
                        first_chunk.ty
                    ),
                ));
            }
            EntryHeader::try_from(first_chunk.data())?
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("`{}` chunk not found", ChunkType::FHED),
            ));
        };
        if header.major != 0 || header.minor != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "entry version {}.{} is not supported",
                    header.major, header.minor
                ),
            ));
        }
        let mut compressed_size = 0;
        let mut extra = vec![];
        let mut data = vec![];
        let mut xattrs = vec![];
        let mut size = None;
        let mut phsf = None;
        let mut ctime = None;
        let mut mtime = None;
        let mut atime = None;
        let mut ctime_ns = None;
        let mut mtime_ns = None;
        let mut atime_ns = None;
        let mut permission = None;
        let mut link_target_type = None;
        let mut owner_uid = None;
        let mut owner_gid = None;
        let mut owner_user_name = None;
        let mut owner_group_name = None;
        let mut owner_user_sid = None;
        let mut owner_group_sid = None;
        let mut permission_mode = None;
        for chunk in chunks {
            match chunk.ty {
                ChunkType::FEND => break,
                ChunkType::PHSF => {
                    phsf = Some(
                        String::from_utf8(chunk.data().into())
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    );
                }
                ChunkType::FDAT => {
                    compressed_size += chunk.data().len();
                    data.push(chunk.data);
                }
                ChunkType::fSIZ => size = Some(u128_from_be_bytes_last(chunk.data())),
                ChunkType::cTIM => ctime = Some(seconds(chunk.data())?),
                ChunkType::mTIM => mtime = Some(seconds(chunk.data())?),
                ChunkType::aTIM => atime = Some(seconds(chunk.data())?),
                ChunkType::cTNS => ctime_ns = Some(nanos(chunk.data())?),
                ChunkType::mTNS => mtime_ns = Some(nanos(chunk.data())?),
                ChunkType::aTNS => atime_ns = Some(nanos(chunk.data())?),
                ChunkType::fPRM => permission = Some(Permission::try_from_bytes(chunk.data())?),
                ChunkType::fUId => owner_uid = Some(OwnerUid::try_from_bytes(chunk.data())?),
                ChunkType::fGId => owner_gid = Some(OwnerGid::try_from_bytes(chunk.data())?),
                ChunkType::fONm => {
                    owner_user_name = Some(OwnerUserName::try_from_bytes(chunk.data())?)
                }
                ChunkType::fGNm => {
                    owner_group_name = Some(OwnerGroupName::try_from_bytes(chunk.data())?)
                }
                ChunkType::fOSi => {
                    owner_user_sid = Some(OwnerUserSid::try_from_bytes(chunk.data())?)
                }
                ChunkType::fGSi => {
                    owner_group_sid = Some(OwnerGroupSid::try_from_bytes(chunk.data())?)
                }
                ChunkType::fMOd => {
                    permission_mode = Some(PermissionMode::try_from_bytes(chunk.data())?)
                }
                ChunkType::xATR => xattrs.push(ExtendedAttribute::try_from_bytes(chunk.data())?),
                ChunkType::fLTP => link_target_type = LinkTargetType::try_from_bytes(chunk.data())?,
                _ => {
                    if chunk.ty.is_critical() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("unknown critical chunk type: `{}`", chunk.ty),
                        ));
                    }
                    extra.push(chunk);
                }
            }
        }
        let ctime = ctime.map(|s| Duration::from_seconds_nanos(s, ctime_ns.unwrap_or(0)));
        let mtime = mtime.map(|s| Duration::from_seconds_nanos(s, mtime_ns.unwrap_or(0)));
        let atime = atime.map(|s| Duration::from_seconds_nanos(s, atime_ns.unwrap_or(0)));

        Ok(Self {
            header,
            phsf,
            extra,
            metadata: Metadata {
                raw_file_size: size,
                compressed_size,
                created: ctime,
                modified: mtime,
                accessed: atime,
                permission,
                link_target_type,
                owner_uid,
                owner_gid,
                owner_user_name,
                owner_group_name,
                owner_user_sid,
                owner_group_sid,
                permission_mode,
                xattrs,
            },
            data,
        })
    }
}

impl<T> SealedEntryExt for NormalEntry<T>
where
    RawChunk<T>: Chunk + Into<RawChunk>,
    (ChunkType, T): Chunk + Into<RawChunk>,
{
    #[allow(deprecated)]
    #[inline]
    fn write_chunks_to<S: EntryChunkSink>(self, sink: &mut S) -> Result<(), S::Error> {
        sink.write_chunk((ChunkType::FHED, self.header.to_bytes()))?;
        for extra_chunk in self.extra {
            sink.write_chunk(extra_chunk)?;
        }
        if let Some(raw_file_size) = self.metadata.raw_file_size {
            let bytes = raw_file_size.to_be_bytes();
            sink.write_chunk((ChunkType::fSIZ, skip_while(&bytes, |i| *i == 0)))?;
        }
        try_for_each_metadata_facet(&self.metadata, |ty, data| sink.write_chunk((ty, data)))?;
        if let Some(phsf) = self.phsf {
            sink.write_chunk((ChunkType::PHSF, phsf.into_bytes()))?;
        }
        for data_chunk in self.data {
            sink.write_chunk((ChunkType::FDAT, data_chunk))?;
        }
        sink.write_chunk((ChunkType::FEND, []))
    }
}

impl<T> Entry for NormalEntry<T> where NormalEntry<T>: SealedEntryExt {}

impl<T> NormalEntry<T> {
    /// Returns the header of this entry.
    #[inline]
    pub fn header(&self) -> &EntryHeader {
        &self.header
    }

    /// Returns the name of this entry.
    ///
    /// # Warning
    ///
    /// The returned name is not sanitized. Using it directly as a filesystem
    /// path may allow path traversal. Call [`EntryName::sanitize`] before
    /// using it as a path.
    #[inline]
    pub fn name(&self) -> &EntryName {
        &self.header.name
    }

    /// Returns the data kind of this entry.
    #[inline]
    pub const fn data_kind(&self) -> DataKind {
        self.header.data_kind
    }

    /// Returns the compression method of this entry.
    #[inline]
    pub const fn compression(&self) -> Compression {
        self.header.compression
    }

    /// Returns the encryption method of this entry.
    #[inline]
    pub const fn encryption(&self) -> Encryption {
        self.header.encryption
    }

    /// Returns the cipher mode of this entry's encryption method.
    #[inline]
    pub const fn cipher_mode(&self) -> CipherMode {
        self.header.cipher_mode
    }

    /// Returns the metadata of this entry.
    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Returns the extended attributes of this entry.
    #[deprecated(
        since = "0.36.0",
        note = "xattrs are now a Metadata facet; use NormalEntry::metadata().xattrs()"
    )]
    #[inline]
    pub fn xattrs(&self) -> &[ExtendedAttribute] {
        self.metadata.xattrs()
    }

    /// Returns the extra chunks of this entry.
    #[inline]
    pub fn extra_chunks(&self) -> &[RawChunk<T>] {
        &self.extra
    }

    /// Applies metadata to the entry.
    ///
    /// # Examples
    /// ```rust
    /// # use std::io;
    /// use libpna::{DirEntryBuilder, Metadata};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut entry = DirEntryBuilder::new("dir_entry".into()).build()?;
    /// entry.with_metadata(Metadata::new());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_metadata(mut self, mut metadata: Metadata) -> Self {
        metadata.compressed_size = self.metadata.compressed_size;
        metadata.raw_file_size = self.metadata.raw_file_size;
        self.metadata = metadata;
        self
    }

    /// Applies extended attributes to the entry.
    #[deprecated(
        since = "0.36.0",
        note = "xattrs are now a Metadata facet; set them via Metadata::with_xattrs and NormalEntry::with_metadata"
    )]
    #[inline]
    pub fn with_xattrs(mut self, xattrs: impl Into<Vec<ExtendedAttribute>>) -> Self {
        self.metadata.xattrs = xattrs.into();
        self
    }

    /// Applies a new name to the entry, preserving all other fields.
    ///
    /// This is useful for path transformations during archive-to-archive copying
    /// where the entry data should remain unchanged.
    ///
    /// # Panics
    ///
    /// Panics when the entry is encrypted in a cipher mode that does not
    /// [allow a header rewrite](CipherMode::allows_header_rewrite), because the
    /// rename would make the entry's data undecryptable. See
    /// [`NormalEntry::try_with_name`] for the fallible variant.
    ///
    /// # Examples
    /// ```rust
    /// # use std::io;
    /// use libpna::DirEntryBuilder;
    ///
    /// # fn main() -> io::Result<()> {
    /// let entry = DirEntryBuilder::new("original/path".into()).build()?;
    /// let renamed = entry.with_name("new/path".into());
    /// assert_eq!(renamed.header().path().as_str(), "new/path");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_name(self, name: EntryName) -> Self {
        self.try_with_name(name)
            .unwrap_or_else(|_| panic!("renaming this entry would make its data undecryptable"))
    }

    /// Returns this entry with a new name, refusing renames that
    /// would make the entry's data undecryptable.
    ///
    /// [`CipherMode::GCM`] derives its stream key from the `FHED` bytes, so a
    /// renamed entry can no longer be decrypted; this method returns an error
    /// instead of producing one. Cipher modes this build does not implement are
    /// refused as well, since their key derivation may bind the header too. See
    /// [`NormalEntry::with_name`] for the panicking variant.
    ///
    /// # Errors
    ///
    /// Returns the entry unchanged when it is encrypted in a cipher mode that
    /// does not [allow a header rewrite](CipherMode::allows_header_rewrite), so
    /// that a caller can copy it verbatim or re-encrypt it instead of having to
    /// ask whether the rename is possible beforehand.
    ///
    /// # Examples
    /// ```rust
    /// # use std::io;
    /// use libpna::DirEntryBuilder;
    ///
    /// # fn main() -> io::Result<()> {
    /// let entry = DirEntryBuilder::new("original/path".into()).build()?;
    /// let Ok(renamed) = entry.try_with_name("new/path".into()) else {
    ///     unreachable!("an unencrypted entry can always be renamed")
    /// };
    /// assert_eq!(renamed.header().path().as_str(), "new/path");
    /// # Ok(())
    /// # }
    /// ```
    // `result_large_err` guards against a large `Err` taxing the common `Ok`
    // path, which cannot happen when both variants are the same type.
    #[allow(clippy::result_large_err)]
    #[inline]
    pub fn try_with_name(mut self, name: EntryName) -> Result<Self, Self> {
        if self.header.encryption != Encryption::NO
            && !self.header.cipher_mode.allows_header_rewrite()
        {
            return Err(self);
        }
        self.header = self.header.with_name(name);
        Ok(self)
    }
}

impl<T: Clone> NormalEntry<T> {
    /// Applies extra chunks to the entry.
    ///
    /// # Examples
    /// ```rust
    /// # use std::io;
    /// use libpna::{ChunkType, DirEntryBuilder, RawChunk};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut entry = DirEntryBuilder::new("dir_entry".into()).build()?;
    /// entry.with_extra_chunks(&[RawChunk::from_data(
    ///     ChunkType::private(*b"myTy").unwrap(),
    ///     b"some data",
    /// )]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_extra_chunks(mut self, chunks: impl Into<Vec<RawChunk<T>>>) -> Self {
        self.extra = chunks.into();
        self
    }
}

impl<T: AsRef<[u8]>> NormalEntry<T> {
    /// Returns a reader over the encoded `FDAT` chunk body bytes.
    ///
    /// This reader exposes the entry data as stored in the archive, before
    /// decryption or decompression. It returns the concatenated bodies of all
    /// `FDAT` chunks and does not include chunk length, type, or CRC bytes.
    #[inline]
    pub fn encoded_reader(&self) -> EncodedDataReader<'_> {
        encoded_data_reader(&self.data)
    }

    /// Returns the reader of this [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while reading from the reader.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use libpna::{Archive, ReadOptions};
    /// use std::{fs, io};
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// for entry in archive.entries().skip_solid() {
    ///     let entry = entry?;
    ///     let mut reader = entry.reader(ReadOptions::builder().build())?;
    ///     let name = entry.header().path();
    ///     let mut dist_file = fs::File::create(name)?;
    ///     io::copy(&mut reader, &mut dist_file)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn reader(&self, option: impl ReadOption) -> io::Result<EntryDataReader<'_>> {
        let decrypt_reader = decrypt_reader(
            self.encoded_reader(),
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            option,
            ChunkType::FHED,
            &self.header.to_bytes(),
        )?;
        let reader = decompress_reader(decrypt_reader, self.header.compression)?;
        Ok(EntryDataReader(EntryReader(reader)))
    }
}

impl<'a> From<NormalEntry<Cow<'a, [u8]>>> for NormalEntry<Vec<u8>> {
    #[inline]
    fn from(value: NormalEntry<Cow<'a, [u8]>>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
        }
    }
}

impl<'a> From<NormalEntry<&'a [u8]>> for NormalEntry<Vec<u8>> {
    #[inline]
    fn from(value: NormalEntry<&'a [u8]>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
        }
    }
}

impl From<NormalEntry<Vec<u8>>> for NormalEntry<Cow<'_, [u8]>> {
    #[inline]
    fn from(value: NormalEntry<Vec<u8>>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
        }
    }
}

impl<'a> From<NormalEntry<&'a [u8]>> for NormalEntry<Cow<'a, [u8]>> {
    #[inline]
    fn from(value: NormalEntry<&'a [u8]>) -> Self {
        Self {
            header: value.header,
            phsf: value.phsf,
            extra: value.extra.into_iter().map(Into::into).collect(),
            data: value.data.into_iter().map(Into::into).collect(),
            metadata: value.metadata,
        }
    }
}

/// A structure representing the split [`Entry`] for archive splitting.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryPart<T = Vec<u8>>(pub(crate) Vec<RawChunk<T>>);

impl<T> EntryPart<T>
where
    RawChunk<T>: Chunk,
{
    /// Returns the total length of this entry part in bytes.
    #[inline]
    pub fn bytes_len(&self) -> usize {
        self.0.iter().map(|chunk| chunk.bytes_len()).sum()
    }

    /// Get reference.
    #[doc(hidden)]
    #[inline]
    pub fn as_ref(&self) -> EntryPart<&[u8]> {
        EntryPart(self.0.iter().map(|it| it.as_ref()).collect())
    }
}

impl EntryPart<&[u8]> {
    /// Splits this [`EntryPart`] into two parts if its length exceeds the given value.
    ///
    /// # Errors
    /// If it can't split into smaller than the given value,
    /// it returns an error containing the original value.
    #[inline]
    pub fn try_split(self, max_bytes_len: usize) -> Result<(Self, Option<Self>), Self> {
        if self.bytes_len() <= max_bytes_len {
            return Ok((self, None));
        }
        let mut remaining = VecDeque::from(self.0);
        let mut first = Vec::new();
        let mut total_size = 0;
        while let Some(chunk) = remaining.pop_front() {
            // NOTE: If over max size, restore to the remaining chunk
            if max_bytes_len < total_size + chunk.bytes_len() {
                if chunk.is_stream_chunk() && total_size + MIN_CHUNK_BYTES_SIZE < max_bytes_len {
                    let available_bytes_len = max_bytes_len - total_size;
                    let chunk_split_index = available_bytes_len - MIN_CHUNK_BYTES_SIZE;
                    let (x, y) = chunk_data_split(chunk.ty, chunk.data, chunk_split_index);
                    first.push(x);
                    if let Some(y) = y {
                        remaining.push_front(y);
                    }
                } else {
                    remaining.push_front(chunk);
                }
                break;
            }
            total_size += chunk.bytes_len();
            first.push(chunk);
        }
        if first.is_empty() {
            return Err(Self(Vec::from(remaining)));
        }
        Ok((Self(first), Some(Self(Vec::from(remaining)))))
    }
}

#[doc(hidden)]
impl<T: SealedEntryExt> From<T> for EntryPart {
    #[inline]
    fn from(value: T) -> Self {
        Self(value.into_chunks())
    }
}

#[inline]
fn seconds(bytes: &[u8]) -> io::Result<i64> {
    Ok(i64::from_be_bytes(bytes.try_into().map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?))
}

#[inline]
fn nanos(bytes: &[u8]) -> io::Result<u32> {
    let v = u32::from_be_bytes(
        bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    );
    if v >= 1_000_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid nanoseconds",
        ));
    }
    Ok(v)
}

#[inline]
fn u128_from_be_bytes_last(bytes: &[u8]) -> u128 {
    const BUF_LEN: usize = std::mem::size_of::<u128>();
    let mut buf = [0u8; BUF_LEN];
    let min = BUF_LEN.min(bytes.len());
    buf[BUF_LEN - min..].copy_from_slice(&bytes[bytes.len() - min..]);
    u128::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn entry_trait_bounds() {
        fn check_impl<T: Entry>() {}
        check_impl::<NormalEntry<Vec<u8>>>();
        check_impl::<NormalEntry<Cow<[u8]>>>();
        check_impl::<NormalEntry<&[u8]>>();
        check_impl::<NormalEntry<[u8; 1]>>();

        check_impl::<SolidEntry<Vec<u8>>>();
        check_impl::<SolidEntry<Cow<[u8]>>>();
        check_impl::<SolidEntry<&[u8]>>();
        check_impl::<SolidEntry<[u8; 1]>>();

        check_impl::<ReadEntry<Vec<u8>>>();
        check_impl::<ReadEntry<Cow<[u8]>>>();
        check_impl::<ReadEntry<&[u8]>>();
        check_impl::<ReadEntry<[u8; 1]>>();

        check_impl::<RawEntry<Vec<u8>>>();
        check_impl::<RawEntry<Cow<[u8]>>>();
        check_impl::<RawEntry<&[u8]>>();
        check_impl::<RawEntry<[u8; 1]>>();
    }

    #[test]
    fn u128_from_be_bytes() {
        assert_eq!(0, u128_from_be_bytes_last(&[]));
        assert_eq!(1, u128_from_be_bytes_last(&[1]));
        assert_eq!(
            u32::MAX as u128,
            u128_from_be_bytes_last(&u32::MAX.to_be_bytes())
        );
        assert_eq!(u128::MAX, u128_from_be_bytes_last(&u128::MAX.to_be_bytes()));
    }

    static TEST_ENTRY: LazyLock<RawEntry> = LazyLock::new(|| {
        RawEntry(vec![
            RawChunk::from_data(
                ChunkType::FHED,
                vec![0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
            ),
            RawChunk::from_data(ChunkType::FDAT, vec![116, 101, 120, 116]),
            RawChunk::from_data(ChunkType::FEND, vec![]),
        ])
    });

    mod entry_part_try_split {
        use super::*;
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        use wasm_bindgen_test::wasm_bindgen_test as test;

        #[test]
        fn split_zero() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            assert_eq!(
                part.as_ref().try_split(0),
                Err(EntryPart::from(entry).as_ref())
            )
        }

        #[test]
        fn bounds_check_spans_unsplittable_chunks() {
            assert_eq!(26, TEST_ENTRY.0.first().unwrap().bytes_len());
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            assert_eq!(
                part.as_ref().try_split(25),
                Err(EntryPart::from(entry).as_ref())
            )
        }

        #[test]
        fn bounds_check_just_end_unsplittable_chunks() {
            assert_eq!(26, TEST_ENTRY.0.first().unwrap().bytes_len());
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());

            assert_eq!(
                part.as_ref().try_split(26),
                Ok((
                    EntryPart(vec![RawChunk::from_slice(
                        ChunkType::FHED,
                        &[0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                    )]),
                    Some(EntryPart(vec![
                        RawChunk::from_slice(ChunkType::FDAT, &[116, 101, 120, 116]),
                        RawChunk::from_slice(ChunkType::FEND, &[]),
                    ]))
                ))
            )
        }

        #[test]
        fn spans_splittable_chunks_below_minimum_chunk_size() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());

            assert_eq!(
                part.as_ref().try_split(27),
                Ok((
                    EntryPart(vec![RawChunk::from_slice(
                        ChunkType::FHED,
                        &[0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                    )]),
                    Some(EntryPart(vec![
                        RawChunk::from_slice(ChunkType::FDAT, &[116, 101, 120, 116]),
                        RawChunk::from_slice(ChunkType::FEND, &[]),
                    ]))
                ))
            )
        }

        #[test]
        fn spans_splittable_chunks() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());

            assert_eq!(
                part.as_ref().try_split(39),
                Ok((
                    EntryPart(vec![
                        RawChunk::from_slice(
                            ChunkType::FHED,
                            &[0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                        ),
                        RawChunk::from_slice(ChunkType::FDAT, &[116]),
                    ]),
                    Some(EntryPart(vec![
                        RawChunk::from_slice(ChunkType::FDAT, &[101, 120, 116]),
                        RawChunk::from_slice(ChunkType::FEND, &[]),
                    ]))
                )),
            )
        }

        #[test]
        fn spans_just_end_of_splittable_chunks() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());

            assert_eq!(
                part.as_ref().try_split(42),
                Ok((
                    EntryPart(vec![
                        RawChunk::from_slice(
                            ChunkType::FHED,
                            &[0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                        ),
                        RawChunk::from_slice(ChunkType::FDAT, &[116, 101, 120, 116]),
                    ]),
                    Some(EntryPart(vec![RawChunk::from_slice(ChunkType::FEND, &[])]))
                ))
            );
        }
    }

    #[test]
    fn normal_entry_with_name_updates_path() {
        let entry = DirEntryBuilder::new("original".into()).build().unwrap();
        let _ = entry.header().path(); // Force cache population
        let renamed = entry.with_name("new".into());
        assert_eq!(renamed.header().path().as_str(), "new");
        assert_eq!(renamed.name().as_str(), "new");
    }

    #[test]
    fn normal_entry_with_name_supports_borrowed_data() {
        let header = EntryHeader::for_file(
            Compression::NO,
            Encryption::NO,
            CipherMode::CTR,
            "original".into(),
        )
        .to_bytes();
        let raw = RawEntry(vec![
            RawChunk::from_slice(ChunkType::FHED, &header),
            RawChunk::from_slice(ChunkType::FEND, &[]),
        ]);
        let entry: NormalEntry<&[u8]> = raw.try_into().unwrap();

        let renamed = entry.with_name("renamed".into());

        assert_eq!(renamed.header().path().as_str(), "renamed");
    }

    #[test]
    #[should_panic(expected = "renaming this entry would make its data undecryptable")]
    fn with_name_panics_on_gcm_encrypted_entry() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::GCM)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .build();
        let mut builder =
            FileEntryBuilder::new_with_options("dir/original".into(), &options).unwrap();
        builder.write_all(b"secret payload").unwrap();
        let entry = builder.build().unwrap();

        let _ = entry.with_name("dir/renamed".into());
    }

    #[test]
    fn try_with_name_refuses_gcm_encrypted_entry() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::GCM)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .build();
        let mut builder =
            FileEntryBuilder::new_with_options("dir/original".into(), &options).unwrap();
        builder.write_all(b"secret payload").unwrap();
        let entry = builder.build().unwrap();

        let refused = entry.try_with_name("dir/renamed".into()).unwrap_err();
        assert_eq!(refused.header().path().as_str(), "dir/original");
        assert_eq!(refused.header().cipher_mode(), CipherMode::GCM);
    }

    #[test]
    fn try_with_name_refuses_an_unsupported_cipher_mode() {
        let mut entry = DirEntryBuilder::new("dir/original".into()).build().unwrap();
        entry.header.encryption = Encryption::AES;
        entry.header.cipher_mode = CipherMode::from_byte(3);

        let refused = entry.try_with_name("dir/renamed".into()).unwrap_err();
        assert_eq!(refused.header().path().as_str(), "dir/original");
        assert_eq!(refused.header().cipher_mode(), CipherMode::from_byte(3));
    }

    #[test]
    fn try_with_name_renames_cbc_entry() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::CBC)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .build();
        let mut builder =
            FileEntryBuilder::new_with_options("dir/original".into(), &options).unwrap();
        builder.write_all(b"secret payload").unwrap();
        let entry = builder.build().unwrap();

        let renamed = entry.try_with_name("dir/renamed".into()).unwrap();
        assert_eq!(renamed.header().path().as_str(), "dir/renamed");
        let mut reader = renamed
            .reader(ReadOptions::with_password(Some("password")))
            .unwrap();
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"secret payload");
    }

    #[test]
    fn cbc_entry_is_readable_after_rename() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::CBC)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .build();
        let mut builder =
            FileEntryBuilder::new_with_options("dir/original".into(), &options).unwrap();
        builder.write_all(b"secret payload").unwrap();
        let entry = builder.build().unwrap();

        let renamed = entry.with_name("dir/renamed".into());
        let mut reader = renamed
            .reader(ReadOptions::with_password(Some("password")))
            .unwrap();
        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        assert_eq!(out, b"secret payload");
    }

    #[test]
    fn normal_entry_encoded_reader_returns_encoded_fdat_body() {
        let data = b"plain data plain data plain data";
        let mut builder = FileEntryBuilder::new_with_options(
            "encoded".into(),
            WriteOptions::builder()
                .compression(Compression::DEFLATE)
                .build(),
        )
        .unwrap();
        builder.write_all(data).unwrap();
        let entry = builder.build().unwrap();

        let mut encoded = Vec::new();
        entry.encoded_reader().read_to_end(&mut encoded).unwrap();

        let mut decoded = Vec::new();
        entry
            .reader(ReadOptions::builder().build())
            .unwrap()
            .read_to_end(&mut decoded)
            .unwrap();

        assert_eq!(decoded, data);
        assert_ne!(encoded, data);
    }

    #[test]
    fn solid_entry_encoded_reader_concatenates_sdat_bodies() {
        let solid = SolidEntry {
            header: SolidHeader::new(Compression::NO, Encryption::NO, CipherMode::CBC),
            phsf: None,
            data: vec![b"abc".to_vec(), b"def".to_vec()],
            extra: Vec::new(),
        };

        let mut encoded = Vec::new();
        solid.encoded_reader().read_to_end(&mut encoded).unwrap();

        assert_eq!(encoded, b"abcdef");
    }

    #[test]
    fn reject_unknown_critical_chunk_in_normal_entry() {
        // Unknown Critical chunk: uppercase first letter = Critical
        let unknown_critical = RawChunk::from_data(
            unsafe { ChunkType::from_unchecked(*b"XUNK") },
            vec![1, 2, 3],
        );
        // Minimal valid FHED: version 0.0, kind=File(0), compression=No(0), encryption=No(0), cipher_mode=0
        let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
        let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

        let raw_entry = RawEntry(vec![fhed, unknown_critical, fend]);
        let result = NormalEntry::try_from(raw_entry);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("critical"));
    }

    #[test]
    fn reject_unknown_critical_chunk_in_solid_entry() {
        let unknown_critical = RawChunk::from_data(
            unsafe { ChunkType::from_unchecked(*b"XUNK") },
            vec![1, 2, 3],
        );
        // Minimal valid SHED: version 0.0, compression=No(0), encryption=No(0), cipher_mode=0
        let shed = RawChunk::from_data(ChunkType::SHED, vec![0, 0, 0, 0, 0]);
        let send = RawChunk::from_data(ChunkType::SEND, vec![]);

        let raw_entry = RawEntry(vec![shed, unknown_critical, send]);
        let result = SolidEntry::try_from(raw_entry);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("critical"));
    }

    #[test]
    fn reject_solid_entry_with_unsupported_major_version() {
        let shed = RawChunk::from_data(ChunkType::SHED, vec![1, 0, 0, 0, 0]);
        let send = RawChunk::from_data(ChunkType::SEND, vec![]);

        let raw_entry = RawEntry(vec![shed, send]);
        let result = SolidEntry::try_from(raw_entry);

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn reject_solid_entry_with_unsupported_minor_version() {
        let shed = RawChunk::from_data(ChunkType::SHED, vec![0, 1, 0, 0, 0]);
        let send = RawChunk::from_data(ChunkType::SEND, vec![]);

        let raw_entry = RawEntry(vec![shed, send]);
        let result = SolidEntry::try_from(raw_entry);

        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn accept_unknown_ancillary_chunk_in_normal_entry() {
        // Unknown Ancillary chunk: lowercase first letter = Ancillary
        let unknown_ancillary = RawChunk::from_data(
            unsafe { ChunkType::from_unchecked(*b"xUNK") },
            vec![1, 2, 3],
        );
        let fhed = RawChunk::from_data(ChunkType::FHED, vec![0, 0, 0, 0, 0, 0]);
        let fend = RawChunk::from_data(ChunkType::FEND, vec![]);

        let raw_entry = RawEntry(vec![fhed, unknown_ancillary, fend]);
        let result = NormalEntry::try_from(raw_entry);

        // Ancillary chunks should be accepted and stored in extra
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.extra.len(), 1);
    }

    #[test]
    fn accept_unknown_ancillary_chunk_in_solid_entry() {
        // Unknown Ancillary chunk: lowercase first letter = Ancillary
        let unknown_ancillary = RawChunk::from_data(
            unsafe { ChunkType::from_unchecked(*b"xUNK") },
            vec![1, 2, 3],
        );
        let shed = RawChunk::from_data(ChunkType::SHED, vec![0, 0, 0, 0, 0]);
        let send = RawChunk::from_data(ChunkType::SEND, vec![]);

        let raw_entry = RawEntry(vec![shed, unknown_ancillary, send]);
        let result = SolidEntry::try_from(raw_entry);

        // Ancillary chunks should be accepted and stored in extra
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.extra.len(), 1);
    }

    #[test]
    fn solid_entry_stops_parsing_at_send() {
        let shed = RawChunk::from_data(ChunkType::SHED, vec![0, 0, 0, 0, 0]);
        let send = RawChunk::from_data(ChunkType::SEND, vec![]);
        // Unknown critical chunk after SEND should be ignored
        let trailing_critical = RawChunk::from_data(
            unsafe { ChunkType::from_unchecked(*b"XUNK") },
            vec![4, 5, 6],
        );

        let raw_entry = RawEntry(vec![shed, send, trailing_critical]);
        let result = SolidEntry::try_from(raw_entry);

        // Should succeed: SEND terminates parsing, trailing chunks are ignored
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.extra.len(), 0);
    }

    fn sample_xattr() -> ExtendedAttribute {
        ExtendedAttribute::new(
            XattrName::try_from("user.k").unwrap(),
            XattrValue::try_from(b"v".as_slice()).unwrap(),
        )
    }

    #[test]
    fn xattrs_round_trip_through_metadata() {
        let xattr = sample_xattr();
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.metadata(Metadata::new().with_xattrs(vec![xattr.clone()]));
        let entry = builder.build().unwrap();
        let restored = NormalEntry::try_from(RawEntry(entry.into_chunks())).unwrap();
        assert_eq!(restored.metadata().xattrs(), &[xattr]);
    }

    #[test]
    fn empty_xattrs_emit_no_xatr_chunk() {
        let entry = FileEntryBuilder::new("f".into()).unwrap().build().unwrap();
        assert!(entry.into_chunks().iter().all(|c| c.ty != ChunkType::xATR));
    }

    #[allow(deprecated)]
    #[test]
    fn deprecated_xattrs_accessor_delegates_to_metadata() {
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.metadata(Metadata::new().with_xattrs(vec![sample_xattr()]));
        let entry = builder.build().unwrap();
        assert_eq!(entry.xattrs(), entry.metadata().xattrs());
        assert!(!entry.xattrs().is_empty());
    }

    #[test]
    fn ancillary_chunks_are_written_before_fdat() {
        let mut builder = FileEntryBuilder::new("f".into()).unwrap();
        builder.metadata(
            Metadata::new()
                .with_created(Some(Duration::seconds(1)))
                .with_permission_mode(Some(PermissionMode::from(0o755)))
                .with_xattrs(vec![sample_xattr()]),
        );
        builder.write_all(b"data").unwrap();
        let entry = builder.build().unwrap();
        let chunks = entry.into_chunks();

        let fdat_pos = chunks.iter().position(|c| c.ty == ChunkType::FDAT).unwrap();
        for ty in [ChunkType::cTIM, ChunkType::fMOd, ChunkType::xATR] {
            let pos = chunks.iter().position(|c| c.ty == ty).unwrap();
            assert!(pos < fdat_pos, "{ty:?} must appear before FDAT");
        }
    }
}
