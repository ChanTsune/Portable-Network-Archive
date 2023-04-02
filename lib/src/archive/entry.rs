mod builder;
mod header;
mod meta;
mod name;
mod options;
mod read;
mod write;

pub use self::{builder::*, header::*, meta::*, name::*, options::*};
use self::{read::*, write::*};
use crate::{
    chunk::{
        chunk_data_split, chunk_to_bytes, ChunkExt, ChunkType, RawChunk, MIN_CHUNK_BYTES_SIZE,
    },
    cipher::{DecryptCbcAes256Reader, DecryptCbcCamellia256Reader},
    hash::verify_password,
};
use std::{
    collections::VecDeque,
    io::{self, Read},
    time::Duration,
};

mod private {
    use super::*;
    pub trait SealedEntry {}
    impl SealedEntry for ReadEntryImpl {}
    impl SealedEntry for ChunkEntry {}
}

/// PNA archive entry
pub trait Entry: private::SealedEntry {
    fn into_bytes(self) -> Vec<u8>;
    fn into_chunks(self) -> Vec<RawChunk>;
}

pub trait ReadEntry: Entry {
    type Reader: Read + Sync + Send;
    fn header(&self) -> &EntryHeader;
    fn metadata(&self) -> &Metadata;
    fn into_reader(self, option: ReadOption) -> io::Result<Self::Reader>;
}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChunkEntry(pub(crate) Vec<RawChunk>);

impl Entry for ChunkEntry {
    fn into_bytes(self) -> Vec<u8> {
        self.0.into_iter().flat_map(chunk_to_bytes).collect()
    }

    #[inline]
    fn into_chunks(self) -> Vec<RawChunk> {
        self.0
    }
}

impl ChunkEntry {
    pub(crate) fn into_entry(self) -> io::Result<ReadEntryImpl> {
        let mut extra = vec![];
        let mut data = vec![];
        let mut info = None;
        let mut phsf = None;
        let mut ctime = None;
        let mut mtime = None;
        let mut permission = None;
        for mut chunk in self.0 {
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
                String::from("FHED chunk not found"),
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
        Ok(ReadEntryImpl {
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
/// [`Read`]
pub struct EntryDataReader(Box<dyn Read + Sync + Send>);

impl Read for EntryDataReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

/// [Entry] that read from PNA archive.
pub(crate) struct ReadEntryImpl {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<RawChunk>,
    pub(crate) data: Vec<u8>,
    pub(crate) metadata: Metadata,
}

impl Entry for ReadEntryImpl {
    fn into_bytes(self) -> Vec<u8> {
        self.into_chunks()
            .into_iter()
            .flat_map(chunk_to_bytes)
            .collect()
    }

    fn into_chunks(self) -> Vec<RawChunk> {
        let mut vec = Vec::new();
        vec.push(RawChunk::from_data(ChunkType::FHED, self.header.to_bytes()));
        if let Some(p) = self.phsf {
            vec.push(RawChunk::from_data(ChunkType::fPRM, p.into_bytes()));
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

impl ReadEntry for ReadEntryImpl {
    type Reader = EntryDataReader;

    #[inline]
    fn header(&self) -> &EntryHeader {
        &self.header
    }

    #[inline]
    fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    #[inline]
    fn into_reader(self, option: ReadOption) -> io::Result<Self::Reader> {
        self.reader(option.password.as_deref())
    }
}

impl ReadEntryImpl {
    fn reader(self, password: Option<&str>) -> io::Result<EntryDataReader> {
        let raw_data_reader = io::Cursor::new(self.data);
        let decrypt_reader: Box<dyn Read + Sync + Send> = match self.header.encryption {
            Encryption::No => Box::new(raw_data_reader),
            encryption @ Encryption::Aes | encryption @ Encryption::Camellia => {
                let s = self.phsf.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        String::from("Item is encrypted, but `PHSF` chunk not found"),
                    )
                })?;
                let phsf = verify_password(
                    &s,
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
                match (encryption, self.header.cipher_mode) {
                    (Encryption::Aes, CipherMode::CBC) => Box::new(DecryptCbcAes256Reader::new(
                        raw_data_reader,
                        hash.as_bytes(),
                    )?),
                    (Encryption::Aes, CipherMode::CTR) => {
                        Box::new(aes_ctr_cipher_reader(raw_data_reader, hash.as_bytes())?)
                    }
                    (Encryption::Camellia, CipherMode::CBC) => Box::new(
                        DecryptCbcCamellia256Reader::new(raw_data_reader, hash.as_bytes())?,
                    ),
                    _ => Box::new(camellia_ctr_cipher_reader(
                        raw_data_reader,
                        hash.as_bytes(),
                    )?),
                }
            }
        };
        let reader: Box<dyn Read + Sync + Send> = match self.header.compression {
            Compression::No => decrypt_reader,
            Compression::Deflate => Box::new(flate2::read::DeflateDecoder::new(decrypt_reader)),
            Compression::ZStandard => Box::new(MutexRead::new(zstd::Decoder::new(decrypt_reader)?)),
            Compression::XZ => Box::new(xz2::read::XzDecoder::new(decrypt_reader)),
        };
        Ok(EntryDataReader(reader))
    }
}

/// A structure representing the [Entry] split for archive splitting.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryPart(pub(crate) Vec<RawChunk>);

impl EntryPart {
    fn bytes_len(&self) -> usize {
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
                if chunk.ty == ChunkType::FDAT && total_size + MIN_CHUNK_BYTES_SIZE < max_bytes_len
                {
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

impl<T: Entry> From<T> for EntryPart {
    fn from(entry: T) -> Self {
        Self(entry.into_chunks())
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
