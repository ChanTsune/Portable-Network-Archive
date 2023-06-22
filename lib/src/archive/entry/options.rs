#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum CipherMode {
    CBC = 0,
    CTR = 1,
}

impl TryFrom<u8> for CipherMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CBC),
            1 => Ok(Self::CTR),
            value => Err(format!("unknown value {value}")),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum HashAlgorithm {
    Pbkdf2Sha256,
    Argon2Id,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WriteOption {
    pub(crate) compression: Compression,
    pub(crate) compression_level: CompressionLevel,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
    pub(crate) hash_algorithm: HashAlgorithm,
    pub(crate) password: Option<String>,
}

impl WriteOption {
    /// A new [WriteOption] to simply store.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::{EntryBuilder, WriteOption};
    ///
    /// EntryBuilder::new_file("example.txt".into(), WriteOption::store()).unwrap();
    /// ```
    ///
    /// [Entry]: crate::Entry
    pub fn store() -> Self {
        Self {
            compression: Compression::No,
            compression_level: Default::default(),
            encryption: Encryption::No,
            cipher_mode: CipherMode::CBC,
            hash_algorithm: HashAlgorithm::Argon2Id,
            password: None,
        }
    }

    #[inline]
    pub fn builder() -> WriteOptionBuilder {
        #[allow(deprecated)]
        WriteOptionBuilder::new()
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WriteOptionBuilder {
    compression: Compression,
    compression_level: CompressionLevel,
    encryption: Encryption,
    cipher_mode: CipherMode,
    hash_algorithm: HashAlgorithm,
    password: Option<String>,
}

impl Default for WriteOptionBuilder {
    fn default() -> Self {
        #[allow(deprecated)]
        Self::new()
    }
}

impl WriteOptionBuilder {
    #[deprecated(since = "0.2.0", note = "Use WriteOption::builder instead.")]
    pub fn new() -> Self {
        Self {
            compression: Compression::No,
            compression_level: CompressionLevel::default(),
            encryption: Encryption::No,
            cipher_mode: CipherMode::CTR,
            hash_algorithm: HashAlgorithm::Argon2Id,
            password: None,
        }
    }

    pub fn compression(&mut self, compression: Compression) -> &mut Self {
        self.compression = compression;
        self
    }

    pub fn compression_level(&mut self, compression_level: CompressionLevel) -> &mut Self {
        self.compression_level = compression_level;
        self
    }

    pub fn encryption(&mut self, encryption: Encryption) -> &mut Self {
        self.encryption = encryption;
        self
    }

    pub fn cipher_mode(&mut self, cipher_mode: CipherMode) -> &mut Self {
        self.cipher_mode = cipher_mode;
        self
    }

    pub fn hash_algorithm(&mut self, algorithm: HashAlgorithm) -> &mut Self {
        self.hash_algorithm = algorithm;
        self
    }

    pub fn password<S: AsRef<str>>(&mut self, password: Option<S>) -> &mut Self {
        self.password = password.map(|it| it.as_ref().to_string());
        self
    }

    pub fn build(&self) -> WriteOption {
        WriteOption {
            compression: self.compression,
            compression_level: self.compression_level,
            encryption: self.encryption,
            cipher_mode: self.cipher_mode,
            hash_algorithm: self.hash_algorithm,
            password: self.password.clone(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ReadOption {
    pub(crate) password: Option<String>,
}

impl ReadOption {
    pub fn builder() -> ReadOptionBuilder {
        ReadOptionBuilder::new()
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
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
