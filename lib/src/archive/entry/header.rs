use super::{CipherMode, Compression, DataKind, Encryption, EntryName};
use std::io;

/// Represents the entry information header that is expressed in the [FHED] chunk.
///
/// [FHED]: crate::ChunkType::FHED
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
    pub(crate) const fn new_with_options(
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

    pub(crate) const fn new(data_kind: DataKind, path: EntryName) -> Self {
        Self::new_with_options(
            data_kind,
            Compression::No,
            Encryption::No,
            CipherMode::CBC,
            path,
        )
    }

    #[inline]
    pub(crate) const fn for_file(
        compression: Compression,
        encryption: Encryption,
        cipher_mode: CipherMode,
        path: EntryName,
    ) -> Self {
        Self::new_with_options(DataKind::File, compression, encryption, cipher_mode, path)
    }

    #[inline]
    pub(crate) const fn for_dir(path: EntryName) -> Self {
        Self::new(DataKind::Directory, path)
    }

    #[inline]
    pub(crate) const fn for_symbolic_link(path: EntryName) -> Self {
        Self::new(DataKind::SymbolicLink, path)
    }

    #[inline]
    pub(crate) const fn for_hard_link(path: EntryName) -> Self {
        Self::new(DataKind::HardLink, path)
    }

    #[inline]
    pub fn path(&self) -> &EntryName {
        &self.path
    }

    #[inline]
    pub const fn data_kind(&self) -> DataKind {
        self.data_kind
    }

    #[inline]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    #[inline]
    pub const fn encryption(&self) -> Encryption {
        self.encryption
    }

    #[inline]
    pub const fn cipher_mode(&self) -> CipherMode {
        self.cipher_mode
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let name = self.path.as_bytes();
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
            path: EntryName::try_from(&bytes[6..])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        })
    }
}

/// Represents the entry information header that is expressed in the [FHED] chunk.
///
/// [FHED]: crate::ChunkType::FHED
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SolidHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
}

impl SolidHeader {
    pub(crate) const fn new(
        compression: Compression,
        encryption: Encryption,
        cipher_mode: CipherMode,
    ) -> Self {
        Self {
            major: 0,
            minor: 0,
            compression,
            encryption,
            cipher_mode,
        }
    }

    #[inline]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    #[inline]
    pub const fn encryption(&self) -> Encryption {
        self.encryption
    }

    #[inline]
    pub const fn cipher_mode(&self) -> CipherMode {
        self.cipher_mode
    }

    #[inline]
    pub const fn to_bytes(&self) -> [u8; 5] {
        [
            self.major,
            self.minor,
            self.compression as u8,
            self.encryption as u8,
            self.cipher_mode as u8,
        ]
    }
}

impl TryFrom<&[u8]> for SolidHeader {
    type Error = io::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            major: bytes[0],
            minor: bytes[1],
            compression: Compression::try_from(bytes[2])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            encryption: Encryption::try_from(bytes[3])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            cipher_mode: CipherMode::try_from(bytes[4])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        })
    }
}
