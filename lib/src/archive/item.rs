use std::io::{self, Read};

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Compression {
    No = 0,
    Deflate = 1,
    ZStandard = 2,
    XZ = 4,
}

impl TryFrom<u8> for Compression {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::Deflate),
            2 => Ok(Self::ZStandard),
            4 => Ok(Self::XZ),
            value => Err(format!("unknown value {value}")),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct CompressionLevel(pub(crate) u8);

impl Default for CompressionLevel {
    #[inline]
    fn default() -> Self {
        Self(u8::MAX)
    }
}

impl From<u8> for CompressionLevel {
    #[inline]
    fn from(value: u8) -> Self {
        Self(value)
    }
}

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Encryption {
    No = 0,
    Aes = 1,
    Camellia = 2,
}

impl TryFrom<u8> for Encryption {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::Aes),
            2 => Ok(Self::Camellia),
            value => Err(format!("unknown value {value}")),
        }
    }
}

#[derive(Copy, Clone)]
pub enum HashAlgorithm {
    Pbkdf2Sha256,
    Argon2Id,
}

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum DataKind {
    File = 0,
    Directory = 1,
    SymbolicLink = 2,
    HardLink = 3,
}

impl TryFrom<u8> for DataKind {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::File),
            1 => Ok(Self::Directory),
            2 => Ok(Self::SymbolicLink),
            3 => Ok(Self::HardLink),
            value => Err(format!("unknown value {value}")),
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub(crate) compression: Compression,
    pub(crate) compression_level: CompressionLevel,
    pub(crate) encryption: Encryption,
    pub(crate) hash_algorithm: HashAlgorithm,
    pub(crate) password: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            compression: Compression::No,
            compression_level: CompressionLevel::default(),
            encryption: Encryption::No,
            hash_algorithm: HashAlgorithm::Argon2Id,
            password: None,
        }
    }
}

impl Options {
    pub fn compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    pub fn compression_level(mut self, compression_level: CompressionLevel) -> Self {
        self.compression_level = compression_level;
        self
    }

    pub fn encryption(mut self, encryption: Encryption) -> Self {
        self.encryption = encryption;
        self
    }

    pub fn hash_algorithm(mut self, algorithm: HashAlgorithm) -> Self {
        self.hash_algorithm = algorithm;
        self
    }

    pub fn password(mut self, password: Option<String>) -> Self {
        self.password = password;
        self
    }
}

pub struct ItemInfo {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) data_kind: DataKind,
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) path: String,
}

pub struct Item {
    pub(crate) info: ItemInfo,
    pub(crate) reader: Box<dyn Read>,
}

impl Read for Item {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Item {
    pub fn path(&self) -> &str {
        &self.info.path
    }
}
