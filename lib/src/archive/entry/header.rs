use super::{CipherMode, Compression, DataKind, Encryption, EntryName};
use std::io;

/// Represents the entry information header that is expressed in the [FHED] chunk.
///
/// [FHED]: crate::FHED
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
    pub fn path(&self) -> &EntryName {
        &self.path
    }

    pub fn data_kind(&self) -> DataKind {
        self.data_kind
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
