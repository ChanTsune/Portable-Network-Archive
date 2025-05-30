use crate::compress;
pub(crate) use private::*;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    str::FromStr,
};

mod private {
    use super::*;

    /// Compression options.
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub enum Compress {
        No,
        Deflate(compress::deflate::DeflateCompressionLevel),
        ZStandard(compress::zstandard::ZstdCompressionLevel),
        XZ(compress::xz::XZCompressionLevel),
    }

    /// Cipher options.
    #[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct Cipher {
        pub(crate) password: Password,
        pub(crate) hash_algorithm: HashAlgorithm,
        pub(crate) cipher_algorithm: CipherAlgorithm,
        pub(crate) mode: CipherMode,
    }

    impl Cipher {
        /// Creates a new [Cipher]
        #[inline]
        pub(crate) const fn new(
            password: Password,
            hash_algorithm: HashAlgorithm,
            cipher_algorithm: CipherAlgorithm,
            mode: CipherMode,
        ) -> Self {
            Self {
                password,
                hash_algorithm,
                cipher_algorithm,
                mode,
            }
        }
    }

    /// Write option getter trait.
    pub trait WriteOption {
        fn compress(&self) -> Compress;
        fn cipher(&self) -> Option<&Cipher>;
        #[inline]
        fn compression(&self) -> Compression {
            match self.compress() {
                Compress::No => Compression::No,
                Compress::Deflate(_) => Compression::Deflate,
                Compress::ZStandard(_) => Compression::ZStandard,
                Compress::XZ(_) => Compression::XZ,
            }
        }

        #[inline]
        fn encryption(&self) -> Encryption {
            self.cipher()
                .map_or(Encryption::No, |it| match it.cipher_algorithm {
                    CipherAlgorithm::Aes => Encryption::Aes,
                    CipherAlgorithm::Camellia => Encryption::Camellia,
                })
        }

        #[inline]
        fn cipher_mode(&self) -> CipherMode {
            self.cipher().map_or(CipherMode::CTR, |it| it.mode)
        }

        #[inline]
        fn hash_algorithm(&self) -> HashAlgorithm {
            self.cipher()
                .map_or_else(HashAlgorithm::argon2id, |it| it.hash_algorithm)
        }

        #[inline]
        fn password(&self) -> Option<&str> {
            self.cipher().map(|it| it.password.0.as_str())
        }
    }

    impl WriteOption for WriteOptions {
        #[inline]
        fn compress(&self) -> Compress {
            self.compress
        }

        #[inline]
        fn cipher(&self) -> Option<&Cipher> {
            self.cipher.as_ref()
        }
    }

    impl<T> WriteOption for &T
    where
        T: WriteOption,
    {
        #[inline]
        fn compress(&self) -> Compress {
            T::compress(self)
        }

        #[inline]
        fn cipher(&self) -> Option<&Cipher> {
            T::cipher(self)
        }
    }

    /// Entry read option getter trait.
    pub trait ReadOption {
        fn password(&self) -> Option<&str>;
    }

    impl<T: ReadOption> ReadOption for &T {
        #[inline]
        fn password(&self) -> Option<&str> {
            T::password(self)
        }
    }

    impl ReadOption for ReadOptions {
        #[inline]
        fn password(&self) -> Option<&str> {
            self.password.as_deref()
        }
    }
}

/// Unknown value error.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UnknownValueError(u8);

impl Display for UnknownValueError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown value {}", self.0)
    }
}

impl Error for UnknownValueError {}

/// Compression method.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(u8)]
pub enum Compression {
    /// Do not apply any compression.
    No = 0,
    /// Zlib format.
    Deflate = 1,
    /// ZStandard format.
    ZStandard = 2,
    /// Xz format.
    XZ = 4,
}

impl TryFrom<u8> for Compression {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::Deflate),
            2 => Ok(Self::ZStandard),
            4 => Ok(Self::XZ),
            value => Err(UnknownValueError(value)),
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
    #[inline]
    pub const fn min() -> Self {
        Self(CompressionLevelImpl::Min)
    }

    /// Maximum compression level.
    /// This value will be replaced with the maximum level for each algorithm.
    #[inline]
    pub const fn max() -> Self {
        Self(CompressionLevelImpl::Max)
    }
}

impl Default for CompressionLevel {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<T: Into<i64>> From<T> for CompressionLevel {
    #[inline]
    fn from(value: T) -> Self {
        Self(CompressionLevelImpl::Custom(value.into()))
    }
}

impl FromStr for CompressionLevel {
    type Err = core::num::ParseIntError;

    /// Parses a string `s` to return a value of this type.
    ///
    /// If parsing succeeds, return the value inside [`Ok`], otherwise
    /// when the string is ill-formatted return an error specific to the
    /// inside [`Err`]. The error type is specific to the implementation of the trait.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::CompressionLevel;
    /// use std::str::FromStr;
    ///
    /// assert_eq!(
    ///     CompressionLevel::min(),
    ///     CompressionLevel::from_str("min").unwrap()
    /// );
    /// assert_eq!(
    ///     CompressionLevel::max(),
    ///     CompressionLevel::from_str("max").unwrap()
    /// );
    /// assert_eq!(
    ///     CompressionLevel::default(),
    ///     CompressionLevel::from_str("default").unwrap()
    /// );
    /// assert_eq!(
    ///     CompressionLevel::from(3),
    ///     CompressionLevel::from_str("3").unwrap()
    /// );
    /// ```
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(CompressionLevelImpl::from_str(s)?))
    }
}

/// Cipher algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum CipherAlgorithm {
    /// Aes algorithm.
    Aes,
    /// Camellia algorithm.
    Camellia,
}

/// Password.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Password(String);

impl Password {
    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl From<String> for Password {
    #[inline]
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Password {
    #[inline]
    fn from(value: &str) -> Self {
        Self(value.into())
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
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::Aes),
            2 => Ok(Self::Camellia),
            value => Err(UnknownValueError(value)),
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
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CBC),
            1 => Ok(Self::CTR),
            value => Err(UnknownValueError(value)),
        }
    }
}

/// Password hash algorithm parameters.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum HashAlgorithmParams {
    /// Pbkdf2 with sha256
    Pbkdf2Sha256 {
        /// Pbkdf2 rounds, if `None` use default rounds.
        rounds: Option<u32>,
    },
    /// Argon2Id
    Argon2Id {
        /// Argon2Id time_cost, if `None` use default time_cost.
        time_cost: Option<u32>,
        /// Argon2Id memory_cost, if `None` use default memory_cost.
        memory_cost: Option<u32>,
        /// Argon2Id parallelism_cost, if `None` use default parallelism_cost.
        parallelism_cost: Option<u32>,
    },
}

/// Password hash algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct HashAlgorithm(pub(crate) HashAlgorithmParams);

impl HashAlgorithm {
    /// Pbkdf2 with sha256
    #[inline]
    pub const fn pbkdf2_sha256() -> Self {
        Self::pbkdf2_sha256_with(None)
    }

    /// Pbkdf2 with sha256 with parameters.
    #[inline]
    pub const fn pbkdf2_sha256_with(rounds: Option<u32>) -> Self {
        Self(HashAlgorithmParams::Pbkdf2Sha256 { rounds })
    }

    /// Argon2Id
    #[inline]
    pub const fn argon2id() -> Self {
        Self::argon2id_with(None, None, None)
    }

    /// Argon2Id with parameters.
    #[inline]
    pub const fn argon2id_with(
        time_cost: Option<u32>,
        memory_cost: Option<u32>,
        parallelism_cost: Option<u32>,
    ) -> Self {
        Self(HashAlgorithmParams::Argon2Id {
            time_cost,
            memory_cost,
            parallelism_cost,
        })
    }
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
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::File),
            1 => Ok(Self::Directory),
            2 => Ok(Self::SymbolicLink),
            3 => Ok(Self::HardLink),
            value => Err(UnknownValueError(value)),
        }
    }
}

/// Options for writing an entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WriteOptions {
    compress: Compress,
    cipher: Option<Cipher>,
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
            compress: Compress::No,
            cipher: None,
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
        let (compression, compression_level) = match value.compress {
            Compress::No => (Compression::No, CompressionLevel::DEFAULT),
            Compress::Deflate(level) => (Compression::Deflate, level.into()),
            Compress::ZStandard(level) => (Compression::ZStandard, level.into()),
            Compress::XZ(level) => (Compression::XZ, level.into()),
        };
        Self {
            compression,
            compression_level,
            encryption: value.encryption(),
            cipher_mode: value.cipher_mode(),
            hash_algorithm: value.hash_algorithm(),
            password: value.password().map(Into::into),
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
            hash_algorithm: HashAlgorithm::argon2id(),
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
        self.password = password.map(|it| it.as_ref().into());
        self
    }

    /// Create new [WriteOptions] parameters set from this builder.
    ///
    /// ## Panics
    ///
    /// Panic will occur when encryption is enabled and a password is not provided.
    #[inline]
    pub fn build(&self) -> WriteOptions {
        let cipher = if self.encryption != Encryption::No {
            Some(Cipher::new(
                self.password
                    .as_deref()
                    .expect("Password was not provided.")
                    .into(),
                self.hash_algorithm,
                match self.encryption {
                    Encryption::Aes => CipherAlgorithm::Aes,
                    Encryption::Camellia => CipherAlgorithm::Camellia,
                    Encryption::No => unreachable!(),
                },
                self.cipher_mode,
            ))
        } else {
            None
        };
        WriteOptions {
            compress: match self.compression {
                Compression::No => Compress::No,
                Compression::Deflate => Compress::Deflate(self.compression_level.into()),
                Compression::ZStandard => Compress::ZStandard(self.compression_level.into()),
                Compression::XZ => Compress::XZ(self.compression_level.into()),
            },
            cipher,
        }
    }
}

/// Options for reading an entry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ReadOptions {
    password: Option<String>,
}

impl ReadOptions {
    /// Create a new [`ReadOptions`] with an optional password.
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
    #[inline]
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
