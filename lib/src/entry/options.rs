use std::str::FromStr;

/// Compression method.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum Compression {
    /// Do not apply any compression.
    No = 0,
    /// Zlib format.
    Deflate = 1,
    /// Zstandard format.
    ZStandard = 2,
    /// Xz format.
    XZ = 4,
}

impl TryFrom<u8> for Compression {
    type Error = String;

    #[inline]
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
pub(crate) enum CompressionLevelImpl {
    /// Minimum compression level.
    Min,
    /// Maximum compression level.
    Max,
    /// Default compression level.
    Default,
    /// Custom compression level.
    Custom(i64),
}

impl FromStr for CompressionLevelImpl {
    type Err = core::num::ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("min") {
            Ok(Self::Min)
        } else if s.eq_ignore_ascii_case("max") {
            Ok(Self::Max)
        } else if s.eq_ignore_ascii_case("default") {
            Ok(Self::Default)
        } else {
            Ok(Self::Custom(i64::from_str(s)?))
        }
    }
}

/// Compression level of each algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct CompressionLevel(pub(crate) CompressionLevelImpl);

impl CompressionLevel {
    pub(crate) const DEFAULT: Self = Self(CompressionLevelImpl::Default);

    /// Minimum compression level.
    /// This value will be replaced with the minimum level for each algorithm.
    pub fn min() -> Self {
        Self(CompressionLevelImpl::Min)
    }

    /// Maximum compression level.
    /// This value will be replaced with the maximum level for each algorithm.
    pub fn max() -> Self {
        Self(CompressionLevelImpl::Max)
    }
}

impl Default for CompressionLevel {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<u8> for CompressionLevel {
    #[inline]
    fn from(value: u8) -> Self {
        Self(CompressionLevelImpl::Custom(i64::from(value)))
    }
}

impl From<u16> for CompressionLevel {
    #[inline]
    fn from(value: u16) -> Self {
        Self(CompressionLevelImpl::Custom(i64::from(value)))
    }
}

impl From<u32> for CompressionLevel {
    #[inline]
    fn from(value: u32) -> Self {
        Self(CompressionLevelImpl::Custom(i64::from(value)))
    }
}

impl From<i8> for CompressionLevel {
    #[inline]
    fn from(value: i8) -> Self {
        Self(CompressionLevelImpl::Custom(i64::from(value)))
    }
}

impl From<i16> for CompressionLevel {
    #[inline]
    fn from(value: i16) -> Self {
        Self(CompressionLevelImpl::Custom(i64::from(value)))
    }
}

impl From<i32> for CompressionLevel {
    #[inline]
    fn from(value: i32) -> Self {
        Self(CompressionLevelImpl::Custom(i64::from(value)))
    }
}

impl FromStr for CompressionLevel {
    type Err = core::num::ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(CompressionLevelImpl::from_str(s)?))
    }
}

/// Encryption algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum Encryption {
    /// Do not apply any encryption.
    No = 0,
    /// Aes algorithm.
    Aes = 1,
    /// Camellia algorithm.
    Camellia = 2,
}

impl TryFrom<u8> for Encryption {
    type Error = String;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::Aes),
            2 => Ok(Self::Camellia),
            value => Err(format!("unknown value {value}")),
        }
    }
}

/// Cipher mode of encryption algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum CipherMode {
    /// Cipher Block Chaining Mode
    CBC = 0,
    /// Counter Mode
    CTR = 1,
}

impl TryFrom<u8> for CipherMode {
    type Error = String;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CBC),
            1 => Ok(Self::CTR),
            value => Err(format!("unknown value {value}")),
        }
    }
}

/// Password hash algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum HashAlgorithm {
    /// Pbkdf2 with sha256
    Pbkdf2Sha256,
    /// Argon2Id
    Argon2Id,
}

/// Type of entry.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum DataKind {
    /// Regular file
    File = 0,
    /// Directory
    Directory = 1,
    /// Symbolic link
    SymbolicLink = 2,
    /// Hard link
    HardLink = 3,
}

impl TryFrom<u8> for DataKind {
    type Error = String;

    #[inline]
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

/// Type alias of [`WriteOptions`].
///
/// This type alias will be removed in the future version.
/// Use [`WriteOptions`] instead.
#[deprecated(
    note = "`WriteOption` was renamed to `WriteOptions`. This type alias will be removed in the future version.",
    since = "0.12.1"
)]
pub type WriteOption = WriteOptions;

/// Options for writing an entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WriteOptions {
    pub(crate) compression: Compression,
    pub(crate) compression_level: CompressionLevel,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
    pub(crate) hash_algorithm: HashAlgorithm,
    pub(crate) password: Option<String>,
}

impl WriteOptions {
    /// A new [WriteOptions] to simply store.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::{EntryBuilder, WriteOptions};
    ///
    /// EntryBuilder::new_file("example.txt".into(), WriteOptions::store()).unwrap();
    /// ```
    ///
    /// [Entry]: crate::Entry
    #[inline]
    pub const fn store() -> Self {
        Self {
            compression: Compression::No,
            compression_level: CompressionLevel::DEFAULT,
            encryption: Encryption::No,
            cipher_mode: CipherMode::CBC,
            hash_algorithm: HashAlgorithm::Argon2Id,
            password: None,
        }
    }

    /// Returns a builder for [WriteOptions].
    ///
    /// # Returns
    ///
    /// [WriteOptionsBuilder] Builder object for [WriteOptions].
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::WriteOptions;
    ///
    /// let builder = WriteOptions::builder();
    /// ```
    #[inline]
    pub const fn builder() -> WriteOptionsBuilder {
        WriteOptionsBuilder::new()
    }

    /// Converts [WriteOptions] into a [WriteOptionsBuilder].
    ///
    /// # Returns
    ///
    /// [WriteOptionsBuilder]: Builder object for [WriteOptions].
    ///
    /// # Examples
    /// ```
    /// use libpna::WriteOptions;
    ///
    /// let write_option = WriteOptions::builder().build();
    /// let builder = write_option.into_builder();
    /// ```
    #[inline]
    pub fn into_builder(self) -> WriteOptionsBuilder {
        self.into()
    }
}

/// Type alias of [`WriteOptionsBuilder`].
///
/// This type alias will be removed in the future version.
/// Use [`WriteOptionsBuilder`] instead.
#[deprecated(
    note = "`WriteOptionBuilder` was renamed to `WriteOptionsBuilder`. This type alias will be removed in the future version.",
    since = "0.12.1"
)]
pub type WriteOptionBuilder = WriteOptionsBuilder;

/// Builder for [`WriteOptions`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WriteOptionsBuilder {
    compression: Compression,
    compression_level: CompressionLevel,
    encryption: Encryption,
    cipher_mode: CipherMode,
    hash_algorithm: HashAlgorithm,
    password: Option<String>,
}

impl Default for WriteOptionsBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<WriteOptions> for WriteOptionsBuilder {
    #[inline]
    fn from(value: WriteOptions) -> Self {
        Self {
            compression: value.compression,
            compression_level: value.compression_level,
            encryption: value.encryption,
            cipher_mode: value.cipher_mode,
            hash_algorithm: value.hash_algorithm,
            password: value.password,
        }
    }
}

impl WriteOptionsBuilder {
    const fn new() -> Self {
        Self {
            compression: Compression::No,
            compression_level: CompressionLevel::DEFAULT,
            encryption: Encryption::No,
            cipher_mode: CipherMode::CTR,
            hash_algorithm: HashAlgorithm::Argon2Id,
            password: None,
        }
    }

    /// Set [Compression] to this builder.
    #[inline]
    pub fn compression(&mut self, compression: Compression) -> &mut Self {
        self.compression = compression;
        self
    }

    /// Set [CompressionLevel] to this builder.
    #[inline]
    pub fn compression_level(&mut self, compression_level: CompressionLevel) -> &mut Self {
        self.compression_level = compression_level;
        self
    }

    /// Set [Encryption] to this builder.
    #[inline]
    pub fn encryption(&mut self, encryption: Encryption) -> &mut Self {
        self.encryption = encryption;
        self
    }

    /// Set [CipherMode] to this builder.
    #[inline]
    pub fn cipher_mode(&mut self, cipher_mode: CipherMode) -> &mut Self {
        self.cipher_mode = cipher_mode;
        self
    }

    /// Set [HashAlgorithm] to this builder.
    #[inline]
    pub fn hash_algorithm(&mut self, algorithm: HashAlgorithm) -> &mut Self {
        self.hash_algorithm = algorithm;
        self
    }

    /// Set the password to this builder.
    #[inline]
    pub fn password<S: AsRef<str>>(&mut self, password: Option<S>) -> &mut Self {
        self.password = password.map(|it| it.as_ref().to_string());
        self
    }

    /// Create new [WriteOptions] parameters set from this builder.
    #[inline]
    pub fn build(&self) -> WriteOptions {
        WriteOptions {
            compression: self.compression,
            compression_level: self.compression_level,
            encryption: self.encryption,
            cipher_mode: self.cipher_mode,
            hash_algorithm: self.hash_algorithm,
            password: self.password.clone(),
        }
    }
}

/// Type alias of [`ReadOptions`].
///
/// This type alias will be removed in the future version.
/// Use [`ReadOptions`] instead.
#[deprecated(
    note = "`ReadOption` was renamed to `ReadOptions`. This type alias will be removed in the future version.",
    since = "0.12.1"
)]
pub type ReadOption = ReadOptions;

/// Options for reading an entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ReadOptions {
    pub(crate) password: Option<String>,
}

impl ReadOptions {
    /// Create a new [`ReadOptions`] with optional password.
    ///
    /// # Examples
    /// ```
    /// use libpna::ReadOptions;
    ///
    /// let read_option = ReadOptions::with_password(Some("password"));
    /// ```
    #[inline]
    pub fn with_password<T: Into<String>>(password: Option<T>) -> Self {
        Self {
            password: password.map(Into::into),
        }
    }

    /// Returns a builder for [ReadOptions].
    ///
    /// # Returns
    ///
    /// [ReadOptionsBuilder]: Builder object for [ReadOptions].
    ///
    /// # Examples
    /// ```
    /// use libpna::ReadOptions;
    ///
    /// let builder = ReadOptions::builder();
    /// ```
    #[inline]
    pub const fn builder() -> ReadOptionsBuilder {
        ReadOptionsBuilder::new()
    }

    /// Converts [ReadOptions] into a [ReadOptionsBuilder].
    ///
    /// # Returns
    ///
    /// [ReadOptionsBuilder]: Builder object for [ReadOptions].
    ///
    /// # Examples
    /// ```
    /// use libpna::ReadOptions;
    ///
    /// let read_option = ReadOptions::builder().build();
    /// let builder = read_option.into_builder();
    /// ```
    #[inline]
    pub fn into_builder(self) -> ReadOptionsBuilder {
        self.into()
    }
}

/// Type alias of [`ReadOptionsBuilder`].
///
/// This type alias will be removed in the future version.
/// Use [`ReadOptionsBuilder`] instead.
#[deprecated(
    note = "`ReadOptionBuilder` was renamed to `ReadOptionsBuilder`. This type alias will be removed in the future version.",
    since = "0.12.1"
)]
pub type ReadOptionBuilder = ReadOptionsBuilder;

/// Builder for [`ReadOptions`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ReadOptionsBuilder {
    password: Option<String>,
}

impl From<ReadOptions> for ReadOptionsBuilder {
    #[inline]
    fn from(value: ReadOptions) -> Self {
        Self {
            password: value.password,
        }
    }
}

impl ReadOptionsBuilder {
    const fn new() -> Self {
        Self { password: None }
    }

    /// Create a new [`ReadOptions`]
    #[inline]
    pub fn build(&self) -> ReadOptions {
        ReadOptions {
            password: self.password.clone(),
        }
    }
}
