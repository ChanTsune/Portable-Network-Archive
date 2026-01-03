use super::{CipherMode, Compression, DataKind, Encryption, EntryName};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::OnceLock;

/// Represents the entry information header that expressed in the [FHED] chunk.
///
/// [FHED]: crate::ChunkType::FHED
#[derive(Clone, Debug)]
pub struct EntryHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) data_kind: DataKind,
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
    sanitized_path: OnceLock<EntryName>,
    pub(crate) name: EntryName,
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
            sanitized_path: OnceLock::new(),
            name: path,
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

    /// Creates a header for a symbolic link (symlink).
    #[inline]
    pub(crate) const fn for_symlink(path: EntryName) -> Self {
        Self::new(DataKind::SymbolicLink, path)
    }

    #[inline]
    pub(crate) const fn for_hard_link(path: EntryName) -> Self {
        Self::new(DataKind::HardLink, path)
    }

    /// Creates a header for a block device.
    #[inline]
    pub(crate) const fn for_block_device(path: EntryName) -> Self {
        Self::new(DataKind::BlockDevice, path)
    }

    /// Creates a header for a character device.
    #[inline]
    pub(crate) const fn for_char_device(path: EntryName) -> Self {
        Self::new(DataKind::CharDevice, path)
    }

    /// Creates a header for a FIFO (named pipe).
    #[inline]
    pub(crate) const fn for_fifo(path: EntryName) -> Self {
        Self::new(DataKind::Fifo, path)
    }

    /// Path of the entry that sanitized to remove path traversal characters by [`EntryName::sanitize`].
    #[inline]
    pub fn path(&self) -> &EntryName {
        self.sanitized_path.get_or_init(|| self.name.sanitize())
    }

    /// Type of the entry.
    #[inline]
    pub const fn data_kind(&self) -> DataKind {
        self.data_kind
    }

    /// Compression method of the entry.
    #[inline]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    /// Encryption method of the entry.
    #[inline]
    pub const fn encryption(&self) -> Encryption {
        self.encryption
    }

    /// Cipher mode of the entry's encryption method.
    #[inline]
    pub const fn cipher_mode(&self) -> CipherMode {
        self.cipher_mode
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let name = self.name.as_bytes();
        let mut data = Vec::with_capacity(6 + name.len());
        data.push(self.major);
        data.push(self.minor);
        data.push(self.data_kind as u8);
        data.push(self.compression as u8);
        data.push(self.encryption as u8);
        data.push(self.cipher_mode as u8);
        data.extend_from_slice(name);
        data
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 6 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "entry header too short",
            ));
        }
        let path = EntryName::from_utf8_preserve_root(
            std::str::from_utf8(&bytes[6..])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        );
        let sanitized = path.sanitize();
        let header = Self {
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
            sanitized_path: OnceLock::new(),
            name: path,
        };
        let _ = header.sanitized_path.set(sanitized);
        Ok(header)
    }
}

impl PartialEq for EntryHeader {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.data_kind == other.data_kind
            && self.compression == other.compression
            && self.encryption == other.encryption
            && self.cipher_mode == other.cipher_mode
            && self.name == other.name
    }
}

impl Eq for EntryHeader {}

impl PartialOrd for EntryHeader {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntryHeader {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.data_kind.cmp(&other.data_kind))
            .then_with(|| self.compression.cmp(&other.compression))
            .then_with(|| self.encryption.cmp(&other.encryption))
            .then_with(|| self.cipher_mode.cmp(&other.cipher_mode))
            .then_with(|| self.path().cmp(other.path()))
            .then_with(|| self.name.cmp(&other.name))
    }
}

impl Hash for EntryHeader {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.major.hash(state);
        self.minor.hash(state);
        self.data_kind.hash(state);
        self.compression.hash(state);
        self.encryption.hash(state);
        self.cipher_mode.hash(state);
        self.path().hash(state);
        self.name.hash(state);
    }
}

impl TryFrom<&[u8]> for EntryHeader {
    type Error = io::Error;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}

/// Represents the entry information header that expressed in the [SHED] chunk.
///
/// [SHED]: crate::ChunkType::SHED
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

    /// Compression method of the solid entry.
    #[inline]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    /// Encryption method of the solid entry.
    #[inline]
    pub const fn encryption(&self) -> Encryption {
        self.encryption
    }

    /// Cipher mode of the solid entry's encryption method.
    #[inline]
    pub const fn cipher_mode(&self) -> CipherMode {
        self.cipher_mode
    }

    /// Converts to [`ChunkType::SHED`](crate::ChunkType::SHED) body bytes.
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

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let bytes: [_; 5] = bytes
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
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

impl TryFrom<&[u8]> for SolidHeader {
    type Error = io::Error;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn entry_header_try_from_bytes() {
        assert!(EntryHeader::try_from_bytes(&[]).is_err());
    }

    #[test]
    fn entry_header_to_from_bytes() {
        let header = EntryHeader::for_file(
            Compression::ZStandard,
            Encryption::Camellia,
            CipherMode::CTR,
            "file".into(),
        );
        assert_eq!(
            header,
            EntryHeader::try_from_bytes(&header.to_bytes()).unwrap(),
        );
    }

    #[test]
    fn solid_header_try_from_bytes() {
        assert!(SolidHeader::try_from_bytes(&[]).is_err());
        assert!(SolidHeader::try_from_bytes(&[0; 5]).is_ok());
    }

    #[test]
    fn solid_header_to_from_bytes() {
        let header = SolidHeader::new(Compression::ZStandard, Encryption::Aes, CipherMode::CBC);
        assert_eq!(
            header,
            SolidHeader::try_from_bytes(&header.to_bytes()).unwrap(),
        );
    }
}
