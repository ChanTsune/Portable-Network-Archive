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
    chunk::{
        chunk_data_split, chunk_to_bytes, ChunkExt, ChunkReader, ChunkType, RawChunk,
        MIN_CHUNK_BYTES_SIZE,
    },
    cipher::{DecryptCbcAes256Reader, DecryptCbcCamellia256Reader, DecryptReader},
    compress::DecompressReader,
    hash::verify_password,
};
use std::{
    collections::VecDeque,
    io::{self, Read},
    time::Duration,
};

mod private {
    use super::*;
    pub trait SealedIntoChunks {
        fn into_chunks(self) -> Vec<RawChunk>;
    }
}

/// Archive entry.
pub trait Entry: SealedIntoChunks {
    fn bytes_len(&self) -> usize;
    fn into_bytes(self) -> Vec<u8>;
}

/// Solid mode entries block.
pub trait SolidEntries: SealedIntoChunks {
    fn bytes_len(&self) -> usize;
    fn into_bytes(self) -> Vec<u8>;
}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChunkEntry(pub(crate) Vec<RawChunk>);

impl SealedIntoChunks for ChunkEntry {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }
}

impl Entry for ChunkEntry {
    fn bytes_len(&self) -> usize {
        self.0.iter().map(|chunk| chunk.bytes_len()).sum()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.into_iter().flat_map(chunk_to_bytes).collect()
    }
}

/// Reader for Entry data. this struct impl [`Read`] trait.
pub struct EntryDataReader(EntryReader<io::Cursor<Vec<u8>>>);

impl Read for EntryDataReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

pub(crate) struct EntryReader<R: Read>(DecompressReader<'static, DecryptReader<R>>);

impl<R: Read> Read for EntryReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum ReadEntryContainer {
    Solid(SolidReadEntry),
    NonSolid(ReadEntry),
}

impl TryFrom<ChunkEntry> for ReadEntryContainer {
    type Error = io::Error;
    fn try_from(entry: ChunkEntry) -> Result<Self, Self::Error> {
        if let Some(first_chunk) = entry.0.first() {
            match first_chunk.ty {
                ChunkType::SHED => Ok(Self::Solid(SolidReadEntry::try_from(entry)?)),
                ChunkType::FHED => Ok(Self::NonSolid(ReadEntry::try_from(entry)?)),
                _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid entry")),
            }
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "Empty entry"))
        }
    }
}

struct EntryIterator<R: Read> {
    entry: EntryReader<R>,
}

impl<R: Read> Iterator for EntryIterator<R> {
    type Item = io::Result<ReadEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk_reader = ChunkReader::from(&mut self.entry);
        let mut chunks = Vec::with_capacity(3);
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
pub(crate) struct SolidReadEntry {
    header: SolidHeader,
    phsf: Option<String>,
    data: Vec<u8>,
    extra: Vec<RawChunk>,
}

impl SolidReadEntry {
    pub(crate) fn entries(
        &self,
        password: Option<&str>,
    ) -> io::Result<impl Iterator<Item = io::Result<ReadEntry>> + '_> {
        let reader = decrypt_reader(
            self.data.as_slice(),
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            password,
        )?;
        let reader = decompress_reader(reader, self.header.compression)?;

        Ok(EntryIterator {
            entry: EntryReader(reader),
        })
    }
}

impl TryFrom<ChunkEntry> for SolidReadEntry {
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
                    data.extend(chunk.data);
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
pub struct ReadEntry {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<RawChunk>,
    pub(crate) data: Vec<u8>,
    pub(crate) metadata: Metadata,
}

impl TryFrom<ChunkEntry> for ReadEntry {
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
        let mut phsf = None;
        let mut ctime = None;
        let mut mtime = None;
        let mut permission = None;
        for mut chunk in entry.0 {
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
                ChunkType::FDAT => data.append(&mut chunk.data),
                ChunkType::cTIM => ctime = Some(timestamp(&chunk.data)?),
                ChunkType::mTIM => mtime = Some(timestamp(&chunk.data)?),
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
                compressed_size: data.len(),
                created: ctime,
                modified: mtime,
                permission,
            },
            data,
        })
    }
}

impl SealedIntoChunks for ReadEntry {
    fn into_chunks(self) -> Vec<RawChunk> {
        let mut vec = Vec::new();
        vec.push(RawChunk::from_data(ChunkType::FHED, self.header.to_bytes()));
        if let Some(p) = self.phsf {
            vec.push(RawChunk::from_data(ChunkType::PHSF, p.into_bytes()));
        }
        for ex in self.extra {
            vec.push(ex);
        }
        for data_chunk in self.data.chunks(u32::MAX as usize) {
            vec.push(RawChunk::from_data(ChunkType::FDAT, data_chunk.to_vec()));
        }
        let Metadata {
            compressed_size: _,
            created,
            modified,
            permission,
        } = self.metadata;
        if let Some(c) = created {
            vec.push(RawChunk::from_data(
                ChunkType::cTIM,
                c.as_secs().to_be_bytes().to_vec(),
            ));
        }
        if let Some(d) = modified {
            vec.push(RawChunk::from_data(
                ChunkType::mTIM,
                d.as_secs().to_be_bytes().to_vec(),
            ));
        }
        if let Some(p) = permission {
            vec.push(RawChunk::from_data(ChunkType::fPRM, p.to_bytes()));
        }
        vec.push(RawChunk::from_data(ChunkType::FEND, Vec::new()));
        vec
    }
}

impl Entry for ReadEntry {
    fn bytes_len(&self) -> usize {
        self.clone().into_bytes().len()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.into_chunks()
            .into_iter()
            .flat_map(chunk_to_bytes)
            .collect()
    }
}

impl ReadEntry {
    #[inline]
    pub fn header(&self) -> &EntryHeader {
        &self.header
    }

    #[inline]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    #[inline]
    pub fn into_reader(self, option: ReadOption) -> io::Result<EntryDataReader> {
        self.reader(option.password.as_deref())
    }

    fn reader(self, password: Option<&str>) -> io::Result<EntryDataReader> {
        let raw_data_reader = io::Cursor::new(self.data);
        let decrypt_reader = decrypt_reader(
            raw_data_reader,
            self.header.encryption,
            self.header.cipher_mode,
            self.phsf.as_deref(),
            password,
        )?;
        let reader = decompress_reader(decrypt_reader, self.header.compression)?;
        Ok(EntryDataReader(EntryReader(reader)))
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
        encryption @ Encryption::Aes | encryption @ Encryption::Camellia => {
            let s = phsf.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    String::from("Item is encrypted, but `PHSF` chunk not found"),
                )
            })?;
            let phsf = verify_password(
                s,
                password.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        String::from("Item is encrypted, but password was not provided"),
                    )
                })?,
            )?;
            let hash = phsf.hash.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Unsupported,
                    String::from("Failed to get hash"),
                )
            })?;
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
fn decompress_reader<'r, R: Read>(
    reader: R,
    compression: Compression,
) -> io::Result<DecompressReader<'r, R>> {
    Ok(match compression {
        Compression::No => DecompressReader::No(reader),
        Compression::Deflate => DecompressReader::Deflate(flate2::read::ZlibDecoder::new(reader)),
        Compression::ZStandard => DecompressReader::ZStd(zstd::Decoder::new(reader)?),
        Compression::XZ => DecompressReader::Xz(xz2::read::XzDecoder::new(reader)),
    })
}

/// A structure representing the [Entry] or the [SolidEntries] split for archive splitting.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryPart(pub(crate) Vec<RawChunk>);

impl EntryPart {
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

impl<T: SealedIntoChunks> From<T> for EntryPart {
    fn from(value: T) -> Self {
        Self(value.into_chunks())
    }
}

pub(crate) struct ChunkSolidEntries(pub(crate) Vec<RawChunk>);

impl SealedIntoChunks for ChunkSolidEntries {
    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }
}

impl SolidEntries for ChunkSolidEntries {
    fn bytes_len(&self) -> usize {
        self.0.iter().map(|chunk| chunk.bytes_len()).sum()
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.into_iter().flat_map(chunk_to_bytes).collect()
    }
}

fn timestamp(bytes: &[u8]) -> io::Result<Duration> {
    Ok(Duration::from_secs(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

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
