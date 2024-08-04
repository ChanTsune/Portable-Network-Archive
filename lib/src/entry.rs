mod attr;
mod builder;
mod header;
mod meta;
mod name;
mod options;
mod read;
mod reference;
mod write;

pub use self::{attr::*, builder::*, header::*, meta::*, name::*, options::*, reference::*};
pub(crate) use self::{private::*, read::*, write::*};
use crate::{
    chunk::{
        chunk_data_split, Chunk, ChunkExt, ChunkReader, ChunkType, RawChunk, MIN_CHUNK_BYTES_SIZE,
    },
    util::slice::skip_while,
};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    time::Duration,
};

mod private {
    use super::*;
    pub trait SealedEntryExt {
        fn into_chunks(self) -> Vec<RawChunk>;
        fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize>;
    }
}

/// Archive entry.
pub trait Entry: SealedEntryExt {}

impl SealedEntryExt for ReadEntry {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        match self {
            Self::Regular(r) => r.into_chunks(),
            Self::Solid(s) => s.into_chunks(),
        }
    }

    #[inline]
    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        match self {
            ReadEntry::Regular(r) => r.write_in(writer),
            ReadEntry::Solid(s) => s.write_in(writer),
        }
    }
}

impl Entry for ReadEntry {}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct RawEntry<T = Vec<u8>>(pub(crate) Vec<RawChunk<T>>);

impl SealedEntryExt for RawEntry {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }

    #[inline]
    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        for chunk in self.0.iter() {
            total += chunk.write_chunk_in(writer)?;
        }
        Ok(total)
    }
}

impl Entry for RawEntry {}

/// Reader for Entry data.
pub struct EntryDataReader<'r>(EntryReader<crate::io::FlattenReader<'r>>);

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

/// A [RegularEntry] or [SolidEntry] read from an archive.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ReadEntry<T = Vec<u8>> {
    /// Solid mode entry
    Solid(SolidEntry<T>),
    /// Regular entry
    Regular(RegularEntry<T>),
}

impl TryFrom<RawEntry> for ReadEntry {
    type Error = io::Error;

    #[inline]
    fn try_from(entry: RawEntry) -> Result<Self, Self::Error> {
        if let Some(first_chunk) = entry.0.first() {
            match first_chunk.ty {
                ChunkType::SHED => Ok(Self::Solid(SolidEntry::try_from(entry)?)),
                ChunkType::FHED => Ok(Self::Regular(RegularEntry::try_from(entry)?)),
                _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid entry")),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "Empty entry"))
        }
    }
}

pub(crate) struct EntryIterator<'s>(EntryReader<crate::io::FlattenReader<'s>>);

impl Iterator for EntryIterator<'_> {
    type Item = io::Result<RegularEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk_reader = ChunkReader::from(&mut self.0);
        let mut chunks = Vec::new();
        loop {
            let chunk = chunk_reader.read_chunk();
            match chunk {
                Ok(chunk) => match chunk.ty {
                    ChunkType::FEND => {
                        chunks.push(chunk);
                        break;
                    }
                    _ => chunks.push(chunk),
                },
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return None,
                Err(e) => return Some(Err(e)),
            }
        }
        Some(RawEntry(chunks).try_into())
    }
}

/// A solid mode entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SolidEntry<T = Vec<u8>> {
    header: SolidHeader,
    phsf: Option<String>,
    data: Vec<T>,
    extra: Vec<RawChunk<T>>,
}

impl SealedEntryExt for SolidEntry<Vec<u8>> {
    fn into_chunks(self) -> Vec<RawChunk> {
        let mut chunks = vec![];
        chunks.push(RawChunk::from_data(ChunkType::SHED, self.header.to_bytes()));
        chunks.extend(self.extra);

        if let Some(phsf) = self.phsf {
            chunks.push(RawChunk::from_data(ChunkType::PHSF, phsf.into_bytes()));
        }
        for data in self.data {
            chunks.push(RawChunk::from_data(ChunkType::SDAT, data));
        }
        chunks.push(RawChunk::from_data(ChunkType::SEND, Vec::new()));
        chunks
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        total += (ChunkType::SHED, self.header.to_bytes().as_slice()).write_chunk_in(writer)?;
        for extra_chunk in &self.extra {
            total += extra_chunk.write_chunk_in(writer)?;
        }
        if let Some(phsf) = &self.phsf {
            total += (ChunkType::PHSF, phsf.as_bytes()).write_chunk_in(writer)?;
        }
        for data in &self.data {
            total += (ChunkType::SDAT, data.as_slice()).write_chunk_in(writer)?;
        }
        total += (ChunkType::SEND, [].as_slice()).write_chunk_in(writer)?;
        Ok(total)
    }
}

impl SealedEntryExt for SolidEntry<&[u8]> {
    fn into_chunks(self) -> Vec<RawChunk> {
        let mut chunks = vec![];
        chunks.push(RawChunk::from_data(ChunkType::SHED, self.header.to_bytes()));
        chunks.extend(self.extra.into_iter().map(|it| it.to_owned()));

        if let Some(phsf) = self.phsf {
            chunks.push(RawChunk::from_data(ChunkType::PHSF, phsf.into_bytes()));
        }
        for data in self.data {
            chunks.push(RawChunk::from_data(ChunkType::SDAT, data));
        }
        chunks.push(RawChunk::from_data(ChunkType::SEND, Vec::new()));
        chunks
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        total += (ChunkType::SHED, self.header.to_bytes().as_slice()).write_chunk_in(writer)?;
        for extra_chunk in &self.extra {
            total += extra_chunk.write_chunk_in(writer)?;
        }
        if let Some(phsf) = &self.phsf {
            total += (ChunkType::PHSF, phsf.as_bytes()).write_chunk_in(writer)?;
        }
        for data in &self.data {
            total += (ChunkType::SDAT, *data).write_chunk_in(writer)?;
        }
        total += (ChunkType::SEND, [].as_slice()).write_chunk_in(writer)?;
        Ok(total)
    }
}

impl<T> Entry for SolidEntry<T> where SolidEntry<T>: SealedEntryExt {}

impl<T> SolidEntry<T> {
    /// Returns solid mode information header reference.
    #[inline]
    pub fn header(&self) -> &SolidHeader {
        &self.header
    }

    /// Extra chunks.
    #[inline]
    pub fn extra_chunks(&self) -> &[RawChunk<T>] {
        &self.extra
    }
}

impl SolidEntry<Vec<u8>> {
    /// Returns an iterator over the entries in the [SolidEntry].
    ///
    /// # Example
    ///
    /// # Example
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
    ///             for entry in solid_entry.entries(Some("password"))? {
    ///                 let entry = entry?;
    ///                 let mut reader = entry.reader(ReadOptions::builder().build());
    ///                 // fill your code
    ///             }
    ///         }
    ///         ReadEntry::Regular(entry) => {
    ///             // fill your code
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn entries(
        &self,
        password: Option<&str>,
    ) -> io::Result<impl Iterator<Item = io::Result<RegularEntry>> + '_> {
        let reader = decrypt_reader(
            crate::io::FlattenReader::new(
                self.data.iter().map(|it| it.iter().as_slice()).collect(),
            ),
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            password.as_ref().map(|it| it.as_bytes()),
        )?;
        let reader = decompress_reader(reader, self.header.compression)?;

        Ok(EntryIterator(EntryReader(reader)))
    }
}

impl SolidEntry<&[u8]> {
    /// Returns an iterator over the entries in the [SolidEntry].
    ///
    /// # Example
    ///
    /// # Example
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
    ///             for entry in solid_entry.entries(Some("password"))? {
    ///                 let entry = entry?;
    ///                 let mut reader = entry.reader(ReadOptions::builder().build());
    ///                 // fill your code
    ///             }
    ///         }
    ///         ReadEntry::Regular(entry) => {
    ///             // fill your code
    ///         }
    ///     }
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn entries(
        &self,
        password: Option<&str>,
    ) -> io::Result<impl Iterator<Item = io::Result<RegularEntry>> + '_> {
        let reader = decrypt_reader(
            crate::io::FlattenReader::new(self.data.clone()),
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            password.as_ref().map(|it| it.as_bytes()),
        )?;
        let reader = decompress_reader(reader, self.header.compression)?;

        Ok(EntryIterator(EntryReader(reader)))
    }
}

impl<T> TryFrom<RawEntry<T>> for SolidEntry<T>
where
    RawChunk<T>: Chunk,
{
    type Error = io::Error;

    #[inline]
    fn try_from(entry: RawEntry<T>) -> Result<Self, Self::Error> {
        if let Some(first_chunk) = entry.0.first() {
            if first_chunk.ty != ChunkType::SHED {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Excepted {} chunk, but {} chunk was found",
                        ChunkType::SHED,
                        first_chunk.ty
                    ),
                ));
            }
        }
        Self::try_from(ChunkSolidEntries(entry.0))
    }
}

impl<T> TryFrom<ChunkSolidEntries<T>> for SolidEntry<T>
where
    RawChunk<T>: Chunk,
{
    type Error = io::Error;

    #[inline]
    fn try_from(entry: ChunkSolidEntries<T>) -> Result<Self, Self::Error> {
        let mut extra = vec![];
        let mut data = vec![];
        let mut info = None;
        let mut phsf = None;
        for chunk in entry.0 {
            match chunk.ty() {
                ChunkType::SHED => {
                    info = Some(SolidHeader::try_from(chunk.data())?);
                }
                ChunkType::SDAT => {
                    data.push(chunk.data);
                }
                ChunkType::PHSF => {
                    phsf = Some(
                        String::from_utf8(chunk.data().into())
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    );
                }
                _ => {
                    extra.push(chunk);
                }
            }
        }
        let header = info.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} chunk not found", ChunkType::SHED),
            )
        })?;
        Ok(Self {
            header,
            phsf,
            data,
            extra,
        })
    }
}

/// [Entry] that read from PNA archive.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RegularEntry<T = Vec<u8>> {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<RawChunk<T>>,
    pub(crate) data: Vec<T>,
    pub(crate) metadata: Metadata,
    pub(crate) xattrs: Vec<ExtendedAttribute>,
}

impl TryFrom<RawEntry> for RegularEntry {
    type Error = io::Error;

    #[inline]
    fn try_from(entry: RawEntry) -> Result<Self, Self::Error> {
        if let Some(first_chunk) = entry.0.first() {
            if first_chunk.ty != ChunkType::FHED {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Excepted {} chunk, but {} chunk was found",
                        ChunkType::FHED,
                        first_chunk.ty
                    ),
                ));
            }
        }
        let mut extra = vec![];
        let mut data = vec![];
        let mut xattrs = vec![];
        let mut info = None;
        let mut size = None;
        let mut phsf = None;
        let mut ctime = None;
        let mut mtime = None;
        let mut atime = None;
        let mut permission = None;
        for chunk in entry.0 {
            match chunk.ty {
                ChunkType::FEND => break,
                ChunkType::FHED => info = Some(EntryHeader::try_from(chunk.data.as_slice())?),
                ChunkType::PHSF => {
                    phsf = Some(
                        String::from_utf8(chunk.data)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    );
                }
                ChunkType::FDAT => data.push(chunk.data),
                ChunkType::fSIZ => size = Some(u128_from_be_bytes_last(&chunk.data)),
                ChunkType::cTIM => ctime = Some(timestamp(&chunk.data)?),
                ChunkType::mTIM => mtime = Some(timestamp(&chunk.data)?),
                ChunkType::aTIM => atime = Some(timestamp(&chunk.data)?),
                ChunkType::fPRM => permission = Some(Permission::try_from_bytes(&chunk.data)?),
                ChunkType::xATR => xattrs.push(ExtendedAttribute::try_from_bytes(&chunk.data)?),
                _ => extra.push(chunk),
            }
        }
        let header = info.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} chunk not found", ChunkType::FHED),
            )
        })?;
        if header.major != 0 || header.minor != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "entry version {}.{} is not supported.",
                    header.major, header.minor
                ),
            ));
        }
        Ok(Self {
            header,
            phsf,
            extra,
            metadata: Metadata {
                raw_file_size: size,
                compressed_size: data.iter().map(|it| it.len()).sum(),
                created: ctime,
                modified: mtime,
                accessed: atime,
                permission,
            },
            data,
            xattrs,
        })
    }
}

impl SealedEntryExt for RegularEntry<Vec<u8>> {
    fn into_chunks(self) -> Vec<RawChunk> {
        let Metadata {
            raw_file_size,
            compressed_size: _,
            created,
            modified,
            accessed,
            permission,
        } = self.metadata;
        let mut vec = Vec::new();
        vec.push(RawChunk::from_data(ChunkType::FHED, self.header.to_bytes()));
        vec.extend(self.extra);
        if let Some(raw_file_size) = raw_file_size {
            vec.push(RawChunk::from_data(
                ChunkType::fSIZ,
                skip_while(&raw_file_size.to_be_bytes(), |i| *i == 0),
            ));
        }

        if let Some(p) = self.phsf {
            vec.push(RawChunk::from_data(ChunkType::PHSF, p.into_bytes()));
        }
        for data_chunk in self.data {
            for data_unit in data_chunk.chunks(u32::MAX as usize) {
                vec.push(RawChunk::from_data(ChunkType::FDAT, data_unit));
            }
        }
        if let Some(c) = created {
            vec.push(RawChunk::from_data(
                ChunkType::cTIM,
                c.as_secs().to_be_bytes(),
            ));
        }
        if let Some(d) = modified {
            vec.push(RawChunk::from_data(
                ChunkType::mTIM,
                d.as_secs().to_be_bytes(),
            ));
        }
        if let Some(a) = accessed {
            vec.push(RawChunk::from_data(
                ChunkType::aTIM,
                a.as_secs().to_be_bytes(),
            ));
        }
        if let Some(p) = permission {
            vec.push(RawChunk::from_data(ChunkType::fPRM, p.to_bytes()));
        }
        for xattr in self.xattrs {
            vec.push(RawChunk::from_data(ChunkType::xATR, xattr.to_bytes()));
        }
        vec.push(RawChunk::from_data(ChunkType::FEND, Vec::new()));
        vec
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;

        let Metadata {
            raw_file_size,
            compressed_size: _,
            created,
            modified,
            accessed,
            permission,
        } = &self.metadata;

        total += (ChunkType::FHED, self.header.to_bytes()).write_chunk_in(writer)?;
        for ex in &self.extra {
            total += ex.write_chunk_in(writer)?;
        }
        if let Some(raw_file_size) = raw_file_size {
            total += (
                ChunkType::fSIZ,
                skip_while(&raw_file_size.to_be_bytes(), |i| *i == 0),
            )
                .write_chunk_in(writer)?;
        }

        if let Some(p) = &self.phsf {
            total += (ChunkType::PHSF, p.as_bytes()).write_chunk_in(writer)?;
        }
        for data_chunk in &self.data {
            for data_unit in data_chunk.chunks(u32::MAX as usize) {
                total += (ChunkType::FDAT, data_unit).write_chunk_in(writer)?;
            }
        }
        if let Some(c) = created {
            total +=
                (ChunkType::cTIM, c.as_secs().to_be_bytes().as_slice()).write_chunk_in(writer)?;
        }
        if let Some(d) = modified {
            total +=
                (ChunkType::mTIM, d.as_secs().to_be_bytes().as_slice()).write_chunk_in(writer)?;
        }
        if let Some(a) = accessed {
            total +=
                (ChunkType::aTIM, a.as_secs().to_be_bytes().as_slice()).write_chunk_in(writer)?;
        }
        if let Some(p) = permission {
            total += (ChunkType::fPRM, p.to_bytes()).write_chunk_in(writer)?;
        }
        for xattr in &self.xattrs {
            total += (ChunkType::xATR, xattr.to_bytes()).write_chunk_in(writer)?;
        }
        total += (ChunkType::FEND, [].as_slice()).write_chunk_in(writer)?;
        Ok(total)
    }
}

impl SealedEntryExt for RegularEntry<&[u8]> {
    fn into_chunks(self) -> Vec<RawChunk> {
        let Metadata {
            raw_file_size,
            compressed_size: _,
            created,
            modified,
            accessed,
            permission,
        } = self.metadata;
        let mut vec = Vec::new();
        vec.push(RawChunk::from_data(ChunkType::FHED, self.header.to_bytes()));
        vec.extend(self.extra.into_iter().map(|it| it.to_owned()));
        if let Some(raw_file_size) = raw_file_size {
            vec.push(RawChunk::from_data(
                ChunkType::fSIZ,
                skip_while(&raw_file_size.to_be_bytes(), |i| *i == 0),
            ));
        }

        if let Some(p) = self.phsf {
            vec.push(RawChunk::from_data(ChunkType::PHSF, p.into_bytes()));
        }
        for data_chunk in self.data {
            for data_unit in data_chunk.chunks(u32::MAX as usize) {
                vec.push(RawChunk::from_data(ChunkType::FDAT, data_unit));
            }
        }
        if let Some(c) = created {
            vec.push(RawChunk::from_data(
                ChunkType::cTIM,
                c.as_secs().to_be_bytes(),
            ));
        }
        if let Some(d) = modified {
            vec.push(RawChunk::from_data(
                ChunkType::mTIM,
                d.as_secs().to_be_bytes(),
            ));
        }
        if let Some(a) = accessed {
            vec.push(RawChunk::from_data(
                ChunkType::aTIM,
                a.as_secs().to_be_bytes(),
            ));
        }
        if let Some(p) = permission {
            vec.push(RawChunk::from_data(ChunkType::fPRM, p.to_bytes()));
        }
        for xattr in self.xattrs {
            vec.push(RawChunk::from_data(ChunkType::xATR, xattr.to_bytes()));
        }
        vec.push(RawChunk::from_data(ChunkType::FEND, Vec::new()));
        vec
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;

        let Metadata {
            raw_file_size,
            compressed_size: _,
            created,
            modified,
            accessed,
            permission,
        } = &self.metadata;

        total += (ChunkType::FHED, self.header.to_bytes()).write_chunk_in(writer)?;
        for ex in &self.extra {
            total += ex.write_chunk_in(writer)?;
        }
        if let Some(raw_file_size) = raw_file_size {
            total += (
                ChunkType::fSIZ,
                skip_while(&raw_file_size.to_be_bytes(), |i| *i == 0),
            )
                .write_chunk_in(writer)?;
        }

        if let Some(p) = &self.phsf {
            total += (ChunkType::PHSF, p.as_bytes()).write_chunk_in(writer)?;
        }
        for data_chunk in &self.data {
            for data_unit in data_chunk.chunks(u32::MAX as usize) {
                total += (ChunkType::FDAT, data_unit).write_chunk_in(writer)?;
            }
        }
        if let Some(c) = created {
            total +=
                (ChunkType::cTIM, c.as_secs().to_be_bytes().as_slice()).write_chunk_in(writer)?;
        }
        if let Some(d) = modified {
            total +=
                (ChunkType::mTIM, d.as_secs().to_be_bytes().as_slice()).write_chunk_in(writer)?;
        }
        if let Some(a) = accessed {
            total +=
                (ChunkType::aTIM, a.as_secs().to_be_bytes().as_slice()).write_chunk_in(writer)?;
        }
        if let Some(p) = permission {
            total += (ChunkType::fPRM, p.to_bytes()).write_chunk_in(writer)?;
        }
        for xattr in &self.xattrs {
            total += (ChunkType::xATR, xattr.to_bytes()).write_chunk_in(writer)?;
        }
        total += (ChunkType::FEND, [].as_slice()).write_chunk_in(writer)?;
        Ok(total)
    }
}

impl<T> Entry for RegularEntry<T> where RegularEntry<T>: SealedEntryExt {}

impl<T> RegularEntry<T> {
    /// Information in the header of the entry.
    #[inline]
    pub fn header(&self) -> &EntryHeader {
        &self.header
    }

    /// Metadata of the entry.
    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Extended attributes of the entry.
    #[inline]
    pub fn xattrs(&self) -> &[ExtendedAttribute] {
        &self.xattrs
    }

    /// Extra chunks.
    #[inline]
    pub fn extra_chunks(&self) -> &[RawChunk<T>] {
        &self.extra
    }

    /// Apply metadata to the entry.
    ///
    /// # Example
    /// ```
    /// # use std::io;
    /// use libpna::{EntryBuilder, Metadata};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut entry = EntryBuilder::new_dir("dir_entry".into()).build()?;
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

    /// Apply extended attributes to the entry.
    ///
    /// # Example
    /// ```
    /// # use std::io;
    /// use libpna::{EntryBuilder, ExtendedAttribute};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut entry = EntryBuilder::new_dir("dir_entry".into()).build()?;
    /// entry.with_xattrs(&[ExtendedAttribute::new("key".into(), b"value".into())]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_xattrs(mut self, xattrs: &[ExtendedAttribute]) -> Self {
        self.xattrs = xattrs.into();
        self
    }
}

impl<T: Clone> RegularEntry<T> {
    /// Apply extra chunks to the entry.
    ///
    /// # Example
    /// ```
    /// # use std::io;
    /// use libpna::{ChunkType, EntryBuilder, RawChunk};
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut entry = EntryBuilder::new_dir("dir_entry".into()).build()?;
    /// entry.with_extra_chunks(&[RawChunk::from_data(
    ///     ChunkType::private(*b"myTy").unwrap(),
    ///     b"some data",
    /// )]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_extra_chunks(mut self, chunks: &[RawChunk<T>]) -> Self {
        self.extra = chunks.to_vec();
        self
    }
}

impl RegularEntry<Vec<u8>> {
    /// Return the reader of this [`RegularEntry`].
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, ReadOptions};
    /// use std::{fs, io};
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// for entry in archive.entries_skip_solid() {
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
    pub fn reader(&self, option: ReadOptions) -> io::Result<EntryDataReader> {
        let raw_data_reader =
            crate::io::FlattenReader::new(self.data.iter().map(|it| it.as_slice()).collect());
        let decrypt_reader = decrypt_reader(
            raw_data_reader,
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            option.password.as_ref().map(|it| it.as_bytes()),
        )?;
        let reader = decompress_reader(decrypt_reader, self.header.compression)?;
        Ok(EntryDataReader(EntryReader(reader)))
    }
}

impl RegularEntry<&[u8]> {
    /// Return the reader of this [`RegularEntry`].
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, ReadOptions};
    /// use std::{fs, io};
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// for entry in archive.entries_skip_solid() {
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
    pub fn reader(&self, option: ReadOptions) -> io::Result<EntryDataReader> {
        let raw_data_reader = crate::io::FlattenReader::new(self.data.clone());
        let decrypt_reader = decrypt_reader(
            raw_data_reader,
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            option.password.as_ref().map(|it| it.as_bytes()),
        )?;
        let reader = decompress_reader(decrypt_reader, self.header.compression)?;
        Ok(EntryDataReader(EntryReader(reader)))
    }
}

/// A structure representing the split [Entry] for archive splitting.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryPart<T = Vec<u8>>(pub(crate) Vec<RawChunk<T>>);

impl<T> EntryPart<T>
where
    RawChunk<T>: Chunk,
{
    /// Length in bytes
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

impl<'a> EntryPart<&'a [u8]> {
    /// Split [EntryPart] into two parts if this entry is shorter in max_bytes_len.
    #[inline]
    pub fn split(self, max_bytes_len: usize) -> (EntryPart<&'a [u8]>, Option<EntryPart<&'a [u8]>>) {
        if self.bytes_len() <= max_bytes_len {
            return (self, None);
        }
        let mut remaining = VecDeque::from(self.0);
        let mut first = vec![];
        let mut total_size = 0;
        while let Some(chunk) = remaining.pop_front() {
            // NOTE: If over max size, restore to remaining chunk
            if max_bytes_len < total_size + chunk.bytes_len() {
                if chunk.is_stream_chunk() && total_size + MIN_CHUNK_BYTES_SIZE < max_bytes_len {
                    let available_bytes_len = max_bytes_len - total_size;
                    let chunk_split_index = available_bytes_len - MIN_CHUNK_BYTES_SIZE;
                    let (x, y) = chunk_data_split(chunk.ty, chunk.data, chunk_split_index);
                    first.push(x);
                    remaining.push_front(y);
                } else {
                    remaining.push_front(chunk);
                }
                break;
            }
            total_size += chunk.bytes_len();
            first.push(chunk);
        }
        (EntryPart(first), Some(EntryPart(Vec::from(remaining))))
    }
}

impl EntryPart {
    /// Split [EntryPart] into two parts if this entry is shorter in max_bytes_len.
    #[inline]
    pub fn split(self, max_bytes_len: usize) -> (EntryPart, Option<EntryPart>) {
        if self.bytes_len() <= max_bytes_len {
            return (self, None);
        }
        let mut remaining = VecDeque::from(self.0);
        let mut first = vec![];
        let mut total_size = 0;
        while let Some(chunk) = remaining.pop_front() {
            // NOTE: If over max size, restore to remaining chunk
            if max_bytes_len < total_size + chunk.bytes_len() {
                if chunk.is_stream_chunk() && total_size + MIN_CHUNK_BYTES_SIZE < max_bytes_len {
                    let available_bytes_len = max_bytes_len - total_size;
                    let chunk_split_index = available_bytes_len - MIN_CHUNK_BYTES_SIZE;
                    let (x, y) = chunk_data_split(chunk.ty, chunk.data(), chunk_split_index);
                    first.push(x.to_owned());
                    remaining.push_front(y.to_owned());
                } else {
                    remaining.push_front(chunk);
                }
                break;
            }
            total_size += chunk.bytes_len();
            first.push(chunk);
        }
        (Self(first), Some(Self(Vec::from(remaining))))
    }
}

#[doc(hidden)]
impl<T: SealedEntryExt> From<T> for EntryPart {
    #[inline]
    fn from(value: T) -> Self {
        Self(value.into_chunks())
    }
}

pub(crate) struct ChunkSolidEntries<T = Vec<u8>>(pub(crate) Vec<RawChunk<T>>);

impl SealedEntryExt for ChunkSolidEntries {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }

    #[inline]
    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        for chunk in self.0.iter() {
            total += chunk.write_chunk_in(writer)?;
        }
        Ok(total)
    }
}

#[inline]
fn timestamp(bytes: &[u8]) -> io::Result<Duration> {
    Ok(Duration::from_secs(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    )))
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

    mod entry_part_split {
        use super::*;
        use once_cell::sync::Lazy;

        static TEST_ENTRY: Lazy<RawEntry> = Lazy::new(|| {
            RawEntry(vec![
                RawChunk::from_data(
                    ChunkType::FHED,
                    vec![0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                ),
                RawChunk::from_data(ChunkType::FDAT, vec![116, 101, 120, 116]),
                RawChunk::from_data(ChunkType::FEND, vec![]),
            ])
        });

        #[test]
        fn split_zero() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            assert_eq!(
                part.split(0),
                (EntryPart(vec![]), Some(EntryPart::from(entry)))
            )
        }

        #[test]
        fn bounds_check_spans_unsplittable_chunks() {
            assert_eq!(26, TEST_ENTRY.0.first().unwrap().bytes_len());
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            let (part1, part2) = part.split(25);

            assert_eq!(0, part1.bytes_len());
            assert_eq!(part2, Some(EntryPart::from(entry)))
        }

        #[test]
        fn bounds_check_just_end_unsplittable_chunks() {
            assert_eq!(26, TEST_ENTRY.0.first().unwrap().bytes_len());
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            let (part1, part2) = part.split(26);

            assert_eq!(26, part1.bytes_len());
            assert_eq!(
                part2,
                Some(EntryPart(vec![
                    RawChunk::from_data(ChunkType::FDAT, vec![116, 101, 120, 116]),
                    RawChunk::from_data(ChunkType::FEND, vec![]),
                ]))
            )
        }

        #[test]
        fn spans_splittable_chunks() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            let (part1, part2) = part.split(39);

            assert_eq!(
                part1,
                EntryPart(vec![
                    RawChunk::from_data(
                        ChunkType::FHED,
                        vec![0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                    ),
                    RawChunk::from_data(ChunkType::FDAT, vec![116]),
                ])
            );
            assert_eq!(
                part2,
                Some(EntryPart(vec![
                    RawChunk::from_data(ChunkType::FDAT, vec![101, 120, 116]),
                    RawChunk::from_data(ChunkType::FEND, vec![]),
                ]))
            )
        }

        #[test]
        fn spans_just_end_of_splittable_chunks() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            let (part1, part2) = part.split(42);

            assert_eq!(
                part1,
                EntryPart(vec![
                    RawChunk::from_data(
                        ChunkType::FHED,
                        vec![0, 0, 0, 0, 0, 1, 116, 101, 115, 116, 46, 116, 120, 116],
                    ),
                    RawChunk::from_data(ChunkType::FDAT, vec![116, 101, 120, 116]),
                ])
            );
            assert_eq!(
                part2,
                Some(EntryPart(vec![RawChunk::from_data(
                    ChunkType::FEND,
                    vec![]
                ),]))
            )
        }

        #[test]
        fn spans_splittable_chunks_below_minimum_chunk_size() {
            let entry = TEST_ENTRY.clone();
            let part = EntryPart::from(entry.clone());
            let (part1, part2) = part.split(27);

            assert_eq!(26, part1.bytes_len());
            assert_eq!(
                part2,
                Some(EntryPart(vec![
                    RawChunk::from_data(ChunkType::FDAT, vec![116, 101, 120, 116]),
                    RawChunk::from_data(ChunkType::FEND, vec![]),
                ]))
            )
        }
    }
}
