mod builder;
mod header;
mod meta;
mod name;
mod options;
mod read;
mod reference;
mod write;

pub use self::{builder::*, header::*, meta::*, name::*, options::*, reference::*};
use self::{private::*, read::*, write::*};
use crate::{
    chunk::{chunk_data_split, ChunkExt, ChunkReader, ChunkType, RawChunk, MIN_CHUNK_BYTES_SIZE},
    cipher::{DecryptCbcAes256Reader, DecryptCbcCamellia256Reader, DecryptReader},
    compress::DecompressReader,
    hash::verify_password,
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
pub trait Entry: SealedEntryExt {
    #[deprecated(since = "0.7.0")]
    fn bytes_len(&self) -> usize;
    #[deprecated(since = "0.7.0")]
    fn into_bytes(self) -> Vec<u8>;
}

impl SealedEntryExt for EntryContainer {
    fn into_chunks(self) -> Vec<RawChunk> {
        match self {
            Self::Regular(r) => r.into_chunks(),
            Self::Solid(s) => s.into_chunks(),
        }
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        match self {
            EntryContainer::Regular(r) => r.write_in(writer),
            EntryContainer::Solid(s) => s.write_in(writer),
        }
    }
}

impl Entry for EntryContainer {
    fn bytes_len(&self) -> usize {
        #[allow(deprecated)]
        match self {
            Self::Regular(r) => r.bytes_len(),
            Self::Solid(s) => s.bytes_len(),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        #[allow(deprecated)]
        match self {
            Self::Regular(r) => r.into_bytes(),
            Self::Solid(s) => s.into_bytes(),
        }
    }
}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChunkEntry(pub(crate) Vec<RawChunk>);

impl SealedEntryExt for ChunkEntry {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        for chunk in self.0.iter() {
            total += chunk.write_in(writer)?;
        }
        Ok(total)
    }
}

impl Entry for ChunkEntry {
    fn bytes_len(&self) -> usize {
        self.0.iter().map(|chunk| chunk.bytes_len()).sum()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.iter().flat_map(ChunkExt::to_bytes).collect()
    }
}

/// Reader for Entry data. this struct impl [`Read`] trait.
pub struct EntryDataReader<'r>(EntryReaderWrapper<'r>);

pub(crate) enum EntryReaderWrapper<'r> {
    Own(EntryReader<crate::io::OwnedFlattenReader>),
    Ref(EntryReader<crate::io::FlattenReader<'r>>),
}

impl<'r> Read for EntryDataReader<'r> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.0 {
            EntryReaderWrapper::Own(s) => s.read(buf),
            EntryReaderWrapper::Ref(s) => s.read(buf),
        }
    }
}

#[cfg(feature = "unstable-async")]
impl<'r> futures::AsyncRead for EntryDataReader<'r> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::task::Poll::Ready(self.get_mut().read(buf))
    }
}

pub(crate) struct EntryReader<R: Read>(DecompressReader<DecryptReader<R>>);

impl<R: Read> Read for EntryReader<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum EntryContainer {
    Solid(SolidEntry),
    Regular(RegularEntry),
}

impl TryFrom<ChunkEntry> for EntryContainer {
    type Error = io::Error;
    fn try_from(entry: ChunkEntry) -> Result<Self, Self::Error> {
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
        Some(ChunkEntry(chunks).try_into())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SolidEntry {
    header: SolidHeader,
    phsf: Option<String>,
    data: Vec<Vec<u8>>,
    extra: Vec<RawChunk>,
}

impl SealedEntryExt for SolidEntry {
    fn into_chunks(self) -> Vec<RawChunk> {
        let mut chunks = vec![];
        chunks.push(RawChunk::from_data(ChunkType::SHED, self.header.to_bytes()));

        if let Some(phsf) = &self.phsf {
            chunks.push(RawChunk::from_data(ChunkType::PHSF, phsf.as_bytes()));
        }
        for data in self.data {
            chunks.push(RawChunk::from_data(ChunkType::SDAT, data));
        }
        chunks.push(RawChunk::from_data(ChunkType::SEND, Vec::new()));
        chunks
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        total += (ChunkType::SHED, self.header.to_bytes().as_slice()).write_in(writer)?;

        if let Some(phsf) = &self.phsf {
            total += (ChunkType::PHSF, phsf.as_bytes()).write_in(writer)?;
        }
        for data in &self.data {
            total += (ChunkType::SDAT, data.as_slice()).write_in(writer)?;
        }
        total += (ChunkType::SEND, [].as_slice()).write_in(writer)?;
        Ok(total)
    }
}

#[allow(deprecated)]
impl SolidEntry {
    fn bytes_len(&self) -> usize {
        self.clone().into_bytes().len()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.into_chunks()
            .iter()
            .flat_map(ChunkExt::to_bytes)
            .collect()
    }
}

impl SolidEntry {
    pub(crate) fn entries(&self, password: Option<&str>) -> io::Result<EntryIterator> {
        let reader = decrypt_reader(
            crate::io::FlattenReader::new(self.data.iter().map(|it| it.as_slice()).collect()),
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            password,
        )?;
        let reader = decompress_reader(reader, self.header.compression)?;

        Ok(EntryIterator(EntryReader(reader)))
    }
}

impl TryFrom<ChunkEntry> for SolidEntry {
    type Error = io::Error;

    fn try_from(entry: ChunkEntry) -> Result<Self, Self::Error> {
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

impl TryFrom<ChunkSolidEntries> for SolidEntry {
    type Error = io::Error;
    fn try_from(entry: ChunkSolidEntries) -> Result<Self, Self::Error> {
        let mut extra = vec![];
        let mut data = vec![];
        let mut info = None;
        let mut phsf = None;
        for chunk in entry.0 {
            match chunk.ty {
                ChunkType::SHED => {
                    info = Some(SolidHeader::try_from(chunk.data.as_slice())?);
                }
                ChunkType::SDAT => {
                    data.push(chunk.data);
                }
                ChunkType::PHSF => {
                    phsf = Some(
                        String::from_utf8(chunk.data)
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
pub struct RegularEntry {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<RawChunk>,
    pub(crate) data: Vec<Vec<u8>>,
    pub(crate) metadata: Metadata,
}

impl TryFrom<ChunkEntry> for RegularEntry {
    type Error = io::Error;
    fn try_from(entry: ChunkEntry) -> Result<Self, Self::Error> {
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
                ChunkType::FHED => {
                    info = Some(EntryHeader::try_from(chunk.data.as_slice())?);
                }
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
        })
    }
}

impl SealedEntryExt for RegularEntry {
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
        if let Some(raw_file_size) = raw_file_size {
            vec.push(RawChunk::from_data(
                ChunkType::fSIZ,
                raw_file_size
                    .to_be_bytes()
                    .into_iter()
                    .skip_while(|i| *i == 0)
                    .collect::<Vec<_>>(),
            ));
        }

        if let Some(p) = self.phsf {
            vec.push(RawChunk::from_data(ChunkType::PHSF, p.into_bytes()));
        }
        for ex in self.extra {
            vec.push(ex);
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

        total += (ChunkType::FHED, self.header.to_bytes()).write_in(writer)?;

        if let Some(raw_file_size) = raw_file_size {
            total += (
                ChunkType::fSIZ,
                skip_while(&raw_file_size.to_be_bytes(), |i| *i == 0),
            )
                .write_in(writer)?;
        }

        if let Some(p) = &self.phsf {
            total += (ChunkType::PHSF, p.as_bytes()).write_in(writer)?;
        }
        for ex in &self.extra {
            total += ex.write_in(writer)?;
        }
        for data_chunk in &self.data {
            for data_unit in data_chunk.chunks(u32::MAX as usize) {
                total += (ChunkType::FDAT, data_unit).write_in(writer)?;
            }
        }
        if let Some(c) = created {
            total += (ChunkType::cTIM, c.as_secs().to_be_bytes().as_slice()).write_in(writer)?;
        }
        if let Some(d) = modified {
            total += (ChunkType::mTIM, d.as_secs().to_be_bytes().as_slice()).write_in(writer)?;
        }
        if let Some(a) = accessed {
            total += (ChunkType::aTIM, a.as_secs().to_be_bytes().as_slice()).write_in(writer)?;
        }
        if let Some(p) = permission {
            total += (ChunkType::fPRM, p.to_bytes()).write_in(writer)?;
        }
        total += (ChunkType::FEND, [].as_slice()).write_in(writer)?;
        Ok(total)
    }
}

impl Entry for RegularEntry {
    #[inline]
    fn bytes_len(&self) -> usize {
        #[allow(deprecated)]
        self.clone().into_bytes().len()
    }

    #[inline]
    fn into_bytes(self) -> Vec<u8> {
        self.into_chunks()
            .iter()
            .flat_map(ChunkExt::to_bytes)
            .collect()
    }
}

impl RegularEntry {
    #[inline]
    pub fn header(&self) -> &EntryHeader {
        &self.header
    }

    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    #[deprecated(since = "0.5.0", note = "Use `RegularEntry::reader` instead.")]
    #[inline]
    pub fn into_reader(self, option: ReadOption) -> io::Result<EntryDataReader<'static>> {
        let raw_data_reader = crate::io::OwnedFlattenReader::new(self.data);
        let decrypt_reader = decrypt_reader(
            raw_data_reader,
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            option.password.as_deref(),
        )?;
        let reader = decompress_reader(decrypt_reader, self.header.compression)?;
        Ok(EntryDataReader(EntryReaderWrapper::Own(EntryReader(
            reader,
        ))))
    }

    /// Return the reader of this [`RegularEntry`].
    ///
    /// # Examples
    /// ```no_run
    /// use libpna::{Archive, ReadOption};
    /// use std::{fs, io};
    ///
    /// # fn main() -> io::Result<()> {
    /// let file = fs::File::open("foo.pna")?;
    /// let mut archive = Archive::read_header(file)?;
    /// for entry in archive.entries_skip_solid() {
    ///     let entry = entry?;
    ///     let mut reader = entry.reader(ReadOption::builder().build())?;
    ///     let name = entry.header().path();
    ///     let mut dist_file = fs::File::create(name)?;
    ///     io::copy(&mut reader, &mut dist_file)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn reader(&self, option: ReadOption) -> io::Result<EntryDataReader> {
        let raw_data_reader =
            crate::io::FlattenReader::new(self.data.iter().map(|it| it.as_slice()).collect());
        let decrypt_reader = decrypt_reader(
            raw_data_reader,
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            option.password.as_deref(),
        )?;
        let reader = decompress_reader(decrypt_reader, self.header.compression)?;
        Ok(EntryDataReader(EntryReaderWrapper::Ref(EntryReader(
            reader,
        ))))
    }
}

/// Decrypt reader according to encryption type.
fn decrypt_reader<R: Read>(
    reader: R,
    encryption: Encryption,
    cipher_mode: CipherMode,
    phsf: Option<&str>,
    password: Option<&str>,
) -> io::Result<DecryptReader<R>> {
    Ok(match encryption {
        Encryption::No => DecryptReader::No(reader),
        encryption @ (Encryption::Aes | Encryption::Camellia) => {
            let s = phsf.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "`PHSF` chunk not found")
            })?;
            let phsf = verify_password(
                s,
                password.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Password was not provided")
                })?,
            )?;
            let hash = phsf
                .hash
                .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "Failed to get hash"))?;
            match (encryption, cipher_mode) {
                (Encryption::Aes, CipherMode::CBC) => {
                    DecryptReader::CbcAes(DecryptCbcAes256Reader::new(reader, hash.as_bytes())?)
                }
                (Encryption::Aes, CipherMode::CTR) => {
                    DecryptReader::CtrAes(aes_ctr_cipher_reader(reader, hash.as_bytes())?)
                }
                (Encryption::Camellia, CipherMode::CBC) => DecryptReader::CbcCamellia(
                    DecryptCbcCamellia256Reader::new(reader, hash.as_bytes())?,
                ),
                _ => {
                    DecryptReader::CtrCamellia(camellia_ctr_cipher_reader(reader, hash.as_bytes())?)
                }
            }
        }
    })
}

/// Decompress reader according to compression type.
fn decompress_reader<R: Read>(
    reader: R,
    compression: Compression,
) -> io::Result<DecompressReader<R>> {
    Ok(match compression {
        Compression::No => DecompressReader::No(reader),
        Compression::Deflate => DecompressReader::Deflate(flate2::read::ZlibDecoder::new(reader)),
        Compression::ZStandard => DecompressReader::ZStd(zstd::Decoder::new(reader)?),
        Compression::XZ => DecompressReader::Xz(liblzma::read::XzDecoder::new(reader)),
    })
}

/// A structure representing the [Entry] or the [SolidEntries] split for archive splitting.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryPart(pub(crate) Vec<RawChunk>);

impl EntryPart {
    #[inline]
    pub fn bytes_len(&self) -> usize {
        self.0.iter().map(|chunk| chunk.bytes_len()).sum()
    }

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
                    let (x, y) = chunk_data_split(chunk, chunk_split_index);
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
        (Self(first), Some(Self(Vec::from(remaining))))
    }
}

impl<T: SealedEntryExt> From<T> for EntryPart {
    fn from(value: T) -> Self {
        Self(value.into_chunks())
    }
}

pub(crate) struct ChunkSolidEntries(pub(crate) Vec<RawChunk>);

impl SealedEntryExt for ChunkSolidEntries {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }

    fn write_in<W: Write>(&self, writer: &mut W) -> io::Result<usize> {
        let mut total = 0;
        for chunk in self.0.iter() {
            total += chunk.write_in(writer)?;
        }
        Ok(total)
    }
}

fn timestamp(bytes: &[u8]) -> io::Result<Duration> {
    Ok(Duration::from_secs(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    )))
}

fn u128_from_be_bytes_last(bytes: &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    for i in 1..=buf.len().min(bytes.len()) {
        buf[buf.len() - i] = bytes[bytes.len() - i];
    }
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
    }

    mod entry_part_split {
        use super::*;
        use once_cell::sync::Lazy;

        static TEST_ENTRY: Lazy<ChunkEntry> = Lazy::new(|| {
            ChunkEntry(vec![
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
