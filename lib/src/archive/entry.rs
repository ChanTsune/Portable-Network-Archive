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
    chunk::{chunk_to_bytes, ChunkType, RawChunk},
    cipher::{DecryptCbcAes256Reader, DecryptCbcCamellia256Reader},
    hash::verify_password,
};
use std::io::{self, Read};
use std::time::Duration;

mod private {
    use super::*;
    pub trait SealedEntry {}
    impl SealedEntry for ReadEntryImpl {}
    impl SealedEntry for ChunkEntry {}
}

/// PNA archive entry
pub trait Entry: private::SealedEntry {
    fn into_bytes(self) -> Vec<u8>;
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
        let mut vec = Vec::new();
        vec.append(&mut chunk_to_bytes((
            ChunkType::FHED,
            self.header.to_bytes(),
        )));
        if let Some(p) = self.phsf {
            vec.append(&mut chunk_to_bytes((ChunkType::fPRM, p.into_bytes())));
        }
        for ex in self.extra {
            vec.append(&mut chunk_to_bytes(ex));
        }
        for data_chunk in self.data.chunks(u32::MAX as usize) {
            vec.append(&mut chunk_to_bytes((ChunkType::FDAT, data_chunk)));
        }
        let Metadata {
            compressed_size: _,
            created,
            modified,
            permission,
        } = self.metadata;
        if let Some(c) = created {
            vec.append(&mut chunk_to_bytes((
                ChunkType::cTIM,
                c.as_secs().to_be_bytes().as_slice(),
            )));
        }
        if let Some(d) = modified {
            vec.append(&mut chunk_to_bytes((
                ChunkType::mTIM,
                d.as_secs().to_be_bytes().as_slice(),
            )));
        }
        if let Some(p) = permission {
            vec.append(&mut chunk_to_bytes((ChunkType::fPRM, p.to_bytes())));
        }
        vec.append(&mut chunk_to_bytes((ChunkType::FEND, [].as_slice())));
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

fn timestamp(bytes: &[u8]) -> io::Result<Duration> {
    Ok(Duration::from_secs(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    )))
}
