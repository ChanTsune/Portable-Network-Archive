mod header;
mod meta;
mod name;
mod options;
mod read;
mod write;

use crate::{
    chunk::{self, chunk_to_bytes, Chunks},
    cipher::{DecryptCbcAes256Reader, DecryptCbcCamellia256Reader},
    hash::verify_password,
};
pub use header::*;
pub use meta::*;
pub use name::*;
pub use options::*;
use read::*;
use std::io::{self, Read, Write};
pub(crate) use write::*;

mod private {
    use super::*;
    pub trait SealedEntry {}
    impl SealedEntry for ReadEntryImpl {}
    impl SealedEntry for ChunkEntry {}
    impl SealedEntry for BytesEntry {}
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
pub(crate) struct ChunkEntry {
    pub(crate) chunks: Chunks,
}

impl Entry for ChunkEntry {
    fn into_bytes(self) -> Vec<u8> {
        self.chunks.into_iter().flat_map(chunk_to_bytes).collect()
    }
}

impl ChunkEntry {
    pub(crate) fn into_entry(self) -> io::Result<ReadEntryImpl> {
        let mut extra = vec![];
        let mut data = vec![];
        let mut info = None;
        let mut phsf = None;
        for (chunk_type, mut raw_data) in self.chunks {
            match chunk_type {
                chunk::FEND => break,
                chunk::FHED => {
                    info = Some(EntryHeader::try_from(raw_data.as_slice())?);
                }
                chunk::PHSF => {
                    phsf = Some(
                        String::from_utf8(raw_data)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    );
                }
                chunk::FDAT => data.append(&mut raw_data),
                _ => extra.push((chunk_type, raw_data)),
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
    pub(crate) extra: Chunks,
    pub(crate) data: Vec<u8>,
    pub(crate) metadata: Metadata,
}

impl Entry for ReadEntryImpl {
    fn into_bytes(self) -> Vec<u8> {
        todo!()
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
                );
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

    pub fn extra(&self) -> &Chunks {
        &self.extra
    }
}

pub struct EntryBuilder(EntryWriter<Vec<u8>>);

impl EntryBuilder {
    pub fn new_file(name: EntryName, option: WriteOption) -> io::Result<Self> {
        Ok(Self(EntryWriter::new_file_with(Vec::new(), name, option)?))
    }

    pub fn build(self) -> io::Result<impl Entry> {
        Ok(BytesEntry(self.0.finish()?))
    }
}

impl Write for EntryBuilder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub(crate) struct BytesEntry(Vec<u8>);

impl Entry for BytesEntry {
    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}
