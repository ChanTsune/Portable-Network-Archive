mod name;
mod options;
mod read;
mod write;

use crate::{
    chunk::{self, from_chunk_data_fhed},
    cipher::{DecryptCbcAes256Reader, DecryptCbcCamellia256Reader},
    hash::verify_password,
    ChunkType,
};
pub use name::*;
pub use options::*;
use read::*;
use std::io::{self, Read};
pub(crate) use write::*;

mod private {
    use super::*;
    pub trait SealedEntry {}
    impl SealedEntry for ReadEntry {}
}

/// PNA archive entry
pub trait Entry: private::SealedEntry {
    type Reader: Read + Sync + Send;
    fn to_reader(self, option: ReadOption) -> io::Result<Self::Reader>;
    fn header(&self) -> &EntryHeader;
}

pub struct ReadOption {
    password: Option<String>,
}

pub struct ReadOptionBuilder {
    password: Option<String>,
}

impl ReadOptionBuilder {
    pub fn new() -> Self {
        Self { password: None }
    }
    pub fn password<T: AsRef<str>>(&mut self, password: T) -> &mut Self {
        self.password = Some(password.as_ref().to_string());
        self
    }
    pub fn build(&self) -> ReadOption {
        ReadOption {
            password: self.password.clone(),
        }
    }
}

pub struct EntryHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) data_kind: DataKind,
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
    pub(crate) path: ItemName,
}

impl EntryHeader {
    pub fn path(&self) -> &ItemName {
        &self.path
    }
}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`
pub(crate) struct RawEntry {
    pub(crate) chunks: Vec<(ChunkType, Vec<u8>)>,
}

impl RawEntry {
    pub(crate) fn into_entry(self) -> io::Result<ReadEntry> {
        let mut extra = vec![];
        let mut data = vec![];
        let mut info = None;
        let mut phsf = None;
        for (chunk_type, mut raw_data) in self.chunks {
            match chunk_type {
                chunk::FEND => break,
                chunk::FHED => {
                    info = Some(from_chunk_data_fhed(&raw_data)?);
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
                    "item version {}.{} is not supported.",
                    header.major, header.minor
                ),
            ));
        }
        Ok(ReadEntry {
            header,
            phsf,
            extra,
            data,
        })
    }
}

/// Entry that read from PNA archive.
pub struct ReadEntry {
    pub(crate) header: EntryHeader,
    pub(crate) phsf: Option<String>,
    pub(crate) extra: Vec<(ChunkType, Vec<u8>)>,
    pub(crate) data: Vec<u8>,
}

pub struct EntryDataReader(Box<dyn Read + Sync + Send>);

impl Read for EntryDataReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Entry for ReadEntry {
    type Reader = EntryDataReader;

    fn to_reader(self, option: ReadOption) -> io::Result<Self::Reader> {
        self.reader(option.password.as_deref())
    }

    fn header(&self) -> &EntryHeader {
        &self.header
    }
}

impl ReadEntry {
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

    pub fn path(&self) -> &str {
        self.header.path.as_ref()
    }

    pub fn kind(&self) -> DataKind {
        self.header.data_kind
    }

    pub fn extra(&self) -> &[(ChunkType, Vec<u8>)] {
        &self.extra
    }
}
