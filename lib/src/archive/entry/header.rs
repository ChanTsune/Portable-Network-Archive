use super::{CipherMode, Compression, DataKind, Encryption, EntryName};
use std::io;

/// Represents the entry information header that is expressed in the [FHED] chunk.
///
/// [FHED]: crate::FHED
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) data_kind: DataKind,
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
    pub(crate) path: EntryName,
}

impl EntryHeader {
    pub(crate) fn new(
        data_kind: DataKind,
        compression: Compression,
        encryption: Encryption,
        cipher_mode: CipherMode,
        path: EntryName,
    ) -> Self {
        Self {
            major: 0,
            minor: 0,
            data_kind,
            compression,
            encryption,
            cipher_mode,
            path,
        }
    }

    #[inline]
    pub(crate) fn for_file(
        compression: Compression,
        encryption: Encryption,
        cipher_mode: CipherMode,
        path: EntryName,
    ) -> Self {
        Self::new(DataKind::File, compression, encryption, cipher_mode, path)
    }

    #[inline]
    pub(crate) fn for_dir(path: EntryName) -> Self {
        Self::new(
            DataKind::Directory,
            Compression::No,
            Encryption::No,
            CipherMode::CBC,
            path,
        )
    }

    pub fn path(&self) -> &EntryName {
        &self.path
    }

    pub fn data_kind(&self) -> DataKind {
        self.data_kind
    }

    pub fn compression(&self) -> Compression {
        self.compression
    }

    pub fn encryption(&self) -> Encryption {
        self.encryption
    }

    pub fn cipher_mode(&self) -> CipherMode {
        self.cipher_mode
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let name = self.path.as_str().as_bytes();
        let mut data = Vec::with_capacity(6 + name.len());
        data.push(self.minor);
        data.push(self.minor);
        data.push(self.data_kind as u8);
        data.push(self.compression as u8);
        data.push(self.encryption as u8);
        data.push(self.cipher_mode as u8);
        data.extend_from_slice(name);
        data
    }
}

impl TryFrom<&[u8]> for EntryHeader {
    type Error = io::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(EntryHeader {
            major: bytes[0],
            minor: bytes[1],
            data_kind: DataKind::try_from(bytes[2])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            compression: Compression::try_from(bytes[3])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            encryption: Encryption::try_from(bytes[4])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            cipher_mode: CipherMode::try_from(bytes[5])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            path: String::from_utf8(bytes[6..].to_vec())
                .map(|s| EntryName::from(&s))
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        })
    }
}
