//! Read and write options for archive entries.

use crate::{compress, entry::write::derive_key_material, error::UnknownValueError};
use password_hash::Output;
pub(crate) use private::*;
use std::{
    collections::HashMap,
    fmt, io,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
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
    #[derive(Clone, Debug)]
    pub struct Cipher {
        pub(crate) password: Password,
        pub(crate) derived: DerivedKeyMaterial,
        pub(crate) hash_algorithm: HashAlgorithm,
        pub(crate) cipher_algorithm: CipherAlgorithm,
        pub(crate) mode: CipherMode,
    }

    impl Cipher {
        /// Creates a new [Cipher].
        #[inline]
        pub(crate) const fn new(
            password: Password,
            derived: DerivedKeyMaterial,
            hash_algorithm: HashAlgorithm,
            cipher_algorithm: CipherAlgorithm,
            mode: CipherMode,
        ) -> Self {
            Self {
                password,
                derived,
                hash_algorithm,
                cipher_algorithm,
                mode,
            }
        }
    }

    /// Maximum number of derived keys retained per cache.
    ///
    /// Archives written after key derivation moved to WriteOptions build time
    /// share a single PHSF across entries, so realistic archives hold only a
    /// few distinct PHSF values. The bound prevents unbounded growth when
    /// reading legacy archives that carry a distinct salt per entry.
    const KEY_CACHE_CAP: usize = 16;

    /// Cache of keys derived from PHC strings.
    ///
    /// Clones share the same underlying storage, so a [`ReadOptions`] and its
    /// clones derive a key at most once per distinct PHC string. Correctness
    /// relies on all sharers holding the same password: [`ReadOptions`] has no
    /// password setter and rebuilding via a builder always starts a new cache.
    #[derive(Clone)]
    pub struct KeyCache {
        inner: Arc<Mutex<HashMap<String, Output>>>,
    }

    impl KeyCache {
        pub(crate) fn new() -> Self {
            Self {
                inner: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub(crate) fn get(&self, phsf: &str) -> Option<Output> {
            self.lock().get(phsf).copied()
        }

        pub(crate) fn insert(&self, phsf: &str, key: Output) {
            let mut map = self.lock();
            if map.len() >= KEY_CACHE_CAP {
                map.clear();
            }
            map.insert(phsf.into(), key);
        }

        fn lock(&self) -> MutexGuard<'_, HashMap<String, Output>> {
            // Critical sections only get/insert; state stays consistent after a
            // poisoning panic, so recover instead of propagating.
            self.inner.lock().unwrap_or_else(PoisonError::into_inner)
        }

        #[cfg(test)]
        pub(crate) fn len(&self) -> usize {
            self.lock().len()
        }
    }

    impl fmt::Debug for KeyCache {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("KeyCache")
                .field("entries", &self.lock().len())
                .finish()
        }
    }

    /// Key material derived from a password when [`WriteOptions`] is built.
    ///
    /// `phsf` is the PHC string (salt and KDF parameters included) recorded in the
    /// PHSF chunk of every entry written with the owning [`WriteOptions`]; `key` is
    /// the KDF output used as the cipher key.
    #[derive(Clone, Debug)]
    pub struct DerivedKeyMaterial {
        pub(crate) phsf: String,
        pub(crate) key: Output,
    }

    /// Accessors for write options.
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
        fn password(&self) -> Option<&[u8]> {
            self.cipher().map(|it| it.password.as_bytes())
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
        fn password(&self) -> Option<&[u8]>;
        fn key_cache(&self) -> Option<&KeyCache>;
    }

    impl<T: ReadOption> ReadOption for &T {
        #[inline]
        fn password(&self) -> Option<&[u8]> {
            T::password(self)
        }

        #[inline]
        fn key_cache(&self) -> Option<&KeyCache> {
            T::key_cache(self)
        }
    }

    impl ReadOption for ReadOptions {
        #[inline]
        fn password(&self) -> Option<&[u8]> {
            self.password.as_deref()
        }

        #[inline]
        fn key_cache(&self) -> Option<&KeyCache> {
            Some(&self.key_cache)
        }
    }
}

/// Compression method.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Compression {
    /// Do not apply any compression.
    No,
    /// Zlib format.
    Deflate,
    /// ZStandard format.
    ZStandard,
    /// Xz format.
    XZ,
    /// Value reserved for future PNA specification (raw value < 128).
    Reserved(u8),
    /// Application-specific private value (raw value >= 128).
    Private(u8),
}

impl Compression {
    /// Serialize this compression method to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::No => 0,
            Self::Deflate => 1,
            Self::ZStandard => 2,
            Self::XZ => 4,
            Self::Reserved(v) | Self::Private(v) => v,
        }
    }

    /// Returns `true` if this is a reserved value.
    #[inline]
    pub const fn is_reserved(self) -> bool {
        matches!(self, Self::Reserved(_))
    }

    /// Returns `true` if this is a private value.
    #[inline]
    pub const fn is_private(self) -> bool {
        matches!(self, Self::Private(_))
    }
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
            v if v < 128 => Ok(Self::Reserved(v)),
            v => Ok(Self::Private(v)),
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
    /// ```rust
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
pub(crate) struct Password(Vec<u8>);

impl Password {
    #[inline]
    pub(crate) const fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<T: AsRef<[u8]>> From<T> for Password {
    #[inline]
    fn from(value: T) -> Self {
        Self(value.as_ref().to_vec())
    }
}

/// Encryption algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Encryption {
    /// Do not apply any encryption.
    No,
    /// Aes algorithm.
    Aes,
    /// Camellia algorithm.
    Camellia,
    /// Value reserved for future PNA specification (raw value < 128).
    Reserved(u8),
    /// Application-specific private value (raw value >= 128).
    Private(u8),
}

impl Encryption {
    /// Serialize this encryption method to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::No => 0,
            Self::Aes => 1,
            Self::Camellia => 2,
            Self::Reserved(v) | Self::Private(v) => v,
        }
    }

    /// Returns `true` if this is a reserved value.
    #[inline]
    pub const fn is_reserved(self) -> bool {
        matches!(self, Self::Reserved(_))
    }

    /// Returns `true` if this is a private value.
    #[inline]
    pub const fn is_private(self) -> bool {
        matches!(self, Self::Private(_))
    }
}

impl TryFrom<u8> for Encryption {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::Aes),
            2 => Ok(Self::Camellia),
            v if v < 128 => Ok(Self::Reserved(v)),
            v => Ok(Self::Private(v)),
        }
    }
}

/// Cipher mode of encryption algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum CipherMode {
    /// Cipher Block Chaining mode.
    CBC,
    /// Counter mode.
    CTR,
    /// Value reserved for future PNA specification (raw value < 128).
    Reserved(u8),
    /// Application-specific private value (raw value >= 128).
    Private(u8),
}

impl CipherMode {
    /// Serialize this cipher mode to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::CBC => 0,
            Self::CTR => 1,
            Self::Reserved(v) | Self::Private(v) => v,
        }
    }

    /// Returns `true` if this is a reserved value.
    #[inline]
    pub const fn is_reserved(self) -> bool {
        matches!(self, Self::Reserved(_))
    }

    /// Returns `true` if this is a private value.
    #[inline]
    pub const fn is_private(self) -> bool {
        matches!(self, Self::Private(_))
    }
}

impl TryFrom<u8> for CipherMode {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::CBC),
            1 => Ok(Self::CTR),
            v if v < 128 => Ok(Self::Reserved(v)),
            v => Ok(Self::Private(v)),
        }
    }
}

/// Password hash algorithm parameters.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum HashAlgorithmParams {
    /// PBKDF2 with SHA-256.
    Pbkdf2Sha256 {
        /// PBKDF2 rounds, if `None` use default rounds.
        rounds: Option<u32>,
    },
    /// Argon2id.
    Argon2Id {
        /// Argon2id time_cost, if `None` use default time_cost.
        time_cost: Option<u32>,
        /// Argon2id memory_cost, if `None` use default memory_cost.
        memory_cost: Option<u32>,
        /// Argon2id parallelism_cost, if `None` use default parallelism_cost.
        parallelism_cost: Option<u32>,
    },
}

/// Password hash algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct HashAlgorithm(pub(crate) HashAlgorithmParams);

impl HashAlgorithm {
    /// Creates a PBKDF2-SHA256 password hasher with default iterations.
    ///
    /// **Note:** Prefer [`argon2id()`](Self::argon2id) for new archives.
    /// PBKDF2 is provided for compatibility with systems where Argon2 is unavailable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{WriteOptions, Encryption, HashAlgorithm};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .hash_algorithm(HashAlgorithm::pbkdf2_sha256())
    ///     .password(Some("password"))
    ///     .build();
    /// ```
    #[inline]
    pub const fn pbkdf2_sha256() -> Self {
        Self::pbkdf2_sha256_with(None)
    }

    /// Creates a PBKDF2-SHA256 password hasher with custom iteration count.
    ///
    /// Higher iteration counts increase security but also increase key derivation time.
    /// If `rounds` is `None`, the default iteration count is used.
    ///
    /// **Note:** Prefer [`argon2id_with()`](Self::argon2id_with) for new archives.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{WriteOptions, Encryption, HashAlgorithm};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(100_000)))
    ///     .password(Some("password"))
    ///     .build();
    /// ```
    #[inline]
    pub const fn pbkdf2_sha256_with(rounds: Option<u32>) -> Self {
        Self(HashAlgorithmParams::Pbkdf2Sha256 { rounds })
    }

    /// Creates an Argon2id password hasher with default parameters.
    ///
    /// **Recommended** for all new archives. Argon2id is memory-hard, providing
    /// better resistance against GPU/ASIC brute-force attacks compared to PBKDF2.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{WriteOptions, Encryption, HashAlgorithm};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .hash_algorithm(HashAlgorithm::argon2id())
    ///     .password(Some("secure_password"))
    ///     .build();
    /// ```
    #[inline]
    pub const fn argon2id() -> Self {
        Self::argon2id_with(None, None, None)
    }

    /// Creates an Argon2id password hasher with custom parameters.
    ///
    /// - `time_cost`: Number of iterations (higher = slower, more secure)
    /// - `memory_cost`: Memory usage in KiB (higher = more memory-hard)
    /// - `parallelism_cost`: Degree of parallelism (threads)
    ///
    /// If any parameter is `None`, the default value is used.
    ///
    /// **Recommended** for all new archives when custom tuning is needed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{WriteOptions, Encryption, HashAlgorithm};
    ///
    /// // Custom Argon2id with higher security parameters
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .hash_algorithm(HashAlgorithm::argon2id_with(
    ///         Some(4),       // time_cost: 4 iterations
    ///         Some(65536),   // memory_cost: 64 MiB
    ///         Some(2),       // parallelism: 2 threads
    ///     ))
    ///     .password(Some("secure_password"))
    ///     .build();
    /// ```
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

/// Type of filesystem object represented by an entry.
///
/// Each variant determines how the entry's data should be interpreted
/// and how the entry should be extracted to the filesystem.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum DataKind {
    /// Regular file. Entry data contains the file contents.
    File,
    /// Directory. Entry has no data content.
    Directory,
    /// Symbolic link. Entry data contains the UTF-8 encoded link target path.
    SymbolicLink,
    /// Hard link. Entry data contains the UTF-8 encoded path of the target entry
    /// within the same archive.
    HardLink,
    /// Value reserved for future PNA specification (raw value < 128).
    Reserved(u8),
    /// Application-specific private value (raw value >= 128).
    Private(u8),
}

impl DataKind {
    /// Serialize this data kind to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        match self {
            Self::File => 0,
            Self::Directory => 1,
            Self::SymbolicLink => 2,
            Self::HardLink => 3,
            Self::Reserved(v) | Self::Private(v) => v,
        }
    }

    /// Returns `true` if this is a reserved value.
    #[inline]
    pub const fn is_reserved(self) -> bool {
        matches!(self, Self::Reserved(_))
    }

    /// Returns `true` if this is a private value.
    #[inline]
    pub const fn is_private(self) -> bool {
        matches!(self, Self::Private(_))
    }
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
            v if v < 128 => Ok(Self::Reserved(v)),
            v => Ok(Self::Private(v)),
        }
    }
}

/// Options for writing entries to a PNA archive.
///
/// This type configures compression, encryption, and password hashing for archive entries.
/// Options are created using the builder pattern via [`WriteOptions::builder()`] or by
/// using the convenience constructor [`WriteOptions::store()`] for uncompressed entries.
///
/// # Compression and Encryption Order
///
/// When both compression and encryption are enabled, data is **compressed first, then encrypted**.
/// This order maximizes compression efficiency since encrypted data is essentially random
/// and cannot be compressed effectively.
///
/// Data flow: `Original → Compress → Encrypt → Write to archive`
///
/// # Security Considerations
///
/// - **Hash Algorithm**: Always use [`HashAlgorithm::argon2id()`] in production for password-based
///   encryption. [`HashAlgorithm::pbkdf2_sha256()`] is primarily for compatibility with older
///   systems or when Argon2 is not available.
/// - **Cipher Mode**: CTR mode ([`CipherMode::CTR`]) is recommended over CBC for most use cases
///   as it allows parallel processing and has simpler security requirements.
/// - **IV Generation**: Initialization vectors (IVs) are automatically generated using
///   cryptographically secure random number generation. You do not need to provide IVs.
/// - **Key Derivation**: The encryption key is derived from the password once when the
///   options are built ([`WriteOptionsBuilder::build()`] / [`WriteOptionsBuilder::try_build()`]),
///   and shared by every entry written with the same [`WriteOptions`]. Each entry still
///   receives a unique, randomly generated IV. Build a fresh [`WriteOptions`] per archive
///   so that each archive uses an independent salt and key.
/// - **Password Strength**: Use strong passwords (12+ characters, mixed case, numbers, symbols)
///   as the encryption key is derived from the password.
///
/// # Examples
///
/// Store without compression or encryption:
/// ```rust
/// use libpna::WriteOptions;
///
/// let opts = WriteOptions::store();
/// ```
///
/// Compress only (no encryption):
/// ```rust
/// use libpna::{WriteOptions, Compression, CompressionLevel};
///
/// let opts = WriteOptions::builder()
///     .compression(Compression::ZStandard)
///     .compression_level(CompressionLevel::max())
///     .build();
/// ```
///
/// Encrypt only (no compression):
/// ```rust
/// use libpna::{WriteOptions, Encryption, CipherMode, HashAlgorithm};
///
/// let opts = WriteOptions::builder()
///     .encryption(Encryption::Aes)
///     .cipher_mode(CipherMode::CTR)
///     .hash_algorithm(HashAlgorithm::argon2id())
///     .password(Some("secure_password"))
///     .build();
/// ```
///
/// Both compression and encryption (recommended for sensitive data):
/// ```rust
/// use libpna::{WriteOptions, Compression, Encryption, CipherMode, HashAlgorithm};
///
/// let opts = WriteOptions::builder()
///     .compression(Compression::ZStandard)
///     .encryption(Encryption::Aes)
///     .cipher_mode(CipherMode::CTR)
///     .hash_algorithm(HashAlgorithm::argon2id())
///     .password(Some("secure_password"))
///     .build();
/// ```
///
/// # Relationship to ReadOptions
///
/// When reading an archive, use [`ReadOptions`] to provide the password for decryption.
/// The compression algorithm and cipher mode are stored in the archive metadata, so you
/// only need to provide the password.
#[derive(Clone, Debug)]
pub struct WriteOptions {
    compress: Compress,
    cipher: Option<Cipher>,
}

impl WriteOptions {
    /// Creates a [`WriteOptions`] that stores data without compression or encryption.
    ///
    /// # Examples
    ///
    /// ```rust
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

    /// Returns a builder for [`WriteOptions`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::WriteOptions;
    ///
    /// let builder = WriteOptions::builder();
    /// ```
    #[inline]
    pub const fn builder() -> WriteOptionsBuilder {
        WriteOptionsBuilder::new()
    }

    /// Converts [`WriteOptions`] into a [`WriteOptionsBuilder`].
    ///
    /// # Examples
    /// ```rust
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
    password: Option<Vec<u8>>,
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
            password: value.password().map(|p| p.to_vec()),
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

    /// Sets the [`Compression`].
    #[inline]
    pub fn compression(&mut self, compression: Compression) -> &mut Self {
        self.compression = compression;
        self
    }

    /// Sets the [`CompressionLevel`].
    #[inline]
    pub fn compression_level(&mut self, compression_level: CompressionLevel) -> &mut Self {
        self.compression_level = compression_level;
        self
    }

    /// Sets the [`Encryption`].
    #[inline]
    pub fn encryption(&mut self, encryption: Encryption) -> &mut Self {
        self.encryption = encryption;
        self
    }

    /// Sets the [`CipherMode`].
    #[inline]
    pub fn cipher_mode(&mut self, cipher_mode: CipherMode) -> &mut Self {
        self.cipher_mode = cipher_mode;
        self
    }

    /// Sets the [`HashAlgorithm`].
    #[inline]
    pub fn hash_algorithm(&mut self, algorithm: HashAlgorithm) -> &mut Self {
        self.hash_algorithm = algorithm;
        self
    }

    /// Sets the password.
    ///
    /// Accepts both UTF-8 strings and arbitrary byte slices.
    ///
    /// # Examples
    /// ```rust
    /// use libpna::WriteOptions;
    ///
    /// // String password
    /// WriteOptions::builder().password(Some("my_password"));
    ///
    /// // Byte slice password
    /// WriteOptions::builder().password(Some(b"binary_password"));
    /// WriteOptions::builder().password(Some(&[0x01, 0x02, 0x03, 0x04]));
    /// ```
    #[inline]
    pub fn password<B: AsRef<[u8]>>(&mut self, password: Option<B>) -> &mut Self {
        self.password = password.map(|it| it.as_ref().to_vec());
        self
    }

    /// Creates a new [`WriteOptions`] from this builder, deriving the encryption
    /// key when encryption is enabled.
    ///
    /// The key derivation function (KDF) runs once here with a freshly generated
    /// random salt. Every entry written with the resulting [`WriteOptions`] shares
    /// the derived key and salt; a unique random IV is still generated per entry.
    ///
    /// # Errors
    ///
    /// - Encryption is enabled but no password was provided.
    /// - The configured KDF parameters are invalid.
    /// - An unsupported encryption or compression method was specified.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{Encryption, WriteOptions};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .password(Some("password"))
    ///     .try_build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "building options without using them is wasteful"]
    pub fn try_build(&self) -> io::Result<WriteOptions> {
        let cipher = if self.encryption != Encryption::No {
            let cipher_algorithm = match self.encryption {
                Encryption::Aes => CipherAlgorithm::Aes,
                Encryption::Camellia => CipherAlgorithm::Camellia,
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!(
                            "unsupported encryption method for writing: byte={}",
                            other.to_byte()
                        ),
                    ));
                }
            };
            let password = self.password.as_deref().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Password was not provided.")
            })?;
            let derived = derive_key_material(cipher_algorithm, self.hash_algorithm, password)?;
            Some(Cipher::new(
                password.into(),
                derived,
                self.hash_algorithm,
                cipher_algorithm,
                self.cipher_mode,
            ))
        } else {
            None
        };
        Ok(WriteOptions {
            compress: match self.compression {
                Compression::No => Compress::No,
                Compression::Deflate => Compress::Deflate(self.compression_level.into()),
                Compression::ZStandard => Compress::ZStandard(self.compression_level.into()),
                Compression::XZ => Compress::XZ(self.compression_level.into()),
                other => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!(
                            "unsupported compression method for writing: byte={}",
                            other.to_byte()
                        ),
                    ));
                }
            },
            cipher,
        })
    }

    /// Creates a new [`WriteOptions`] from this builder.
    ///
    /// This finalizes the builder configuration and creates an immutable [`WriteOptions`]
    /// that can be used when creating entries.
    ///
    /// # Panics
    ///
    /// Panics if [`encryption()`](Self::encryption) was set to [`Encryption::Aes`] or
    /// [`Encryption::Camellia`] but [`password()`](Self::password) was not called with
    /// a password, or if key derivation fails (see [`try_build()`](Self::try_build) for
    /// the fallible variant).
    ///
    /// **Always provide a password when enabling encryption.** The following code will panic:
    /// ```no_run
    /// use libpna::{WriteOptions, Encryption};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .build();  // PANICS: "Password was not provided."
    /// ```
    ///
    /// **Correct usage:**
    /// ```rust
    /// use libpna::{WriteOptions, Encryption};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::Aes)
    ///     .password(Some("secure_password"))
    ///     .build();  // OK
    /// ```
    #[inline]
    #[must_use = "building options without using them is wasteful"]
    pub fn build(&self) -> WriteOptions {
        match self.try_build() {
            Ok(options) => options,
            Err(e) => panic!("{e}"),
        }
    }
}

/// Options for reading an entry.
///
/// Derived encryption keys are cached inside the options and shared between
/// clones: reading many entries that carry the same PHC string (the default
/// for archives written by this crate) runs the key derivation function only
/// once. Rebuilding via [`ReadOptions::into_builder`] always starts with an
/// empty cache.
#[derive(Clone, Debug)]
pub struct ReadOptions {
    password: Option<Vec<u8>>,
    key_cache: KeyCache,
}

impl ReadOptions {
    /// Creates a new [`ReadOptions`] with an optional password.
    ///
    /// Accepts both UTF-8 strings and arbitrary byte slices.
    ///
    /// # Examples
    /// ```rust
    /// use libpna::ReadOptions;
    ///
    /// // String password
    /// let read_option = ReadOptions::with_password(Some("password"));
    ///
    /// // Byte slice password
    /// let read_option = ReadOptions::with_password(Some(b"password"));
    /// let read_option = ReadOptions::with_password(Some(&[0x01, 0x02, 0x03]));
    /// ```
    #[inline]
    pub fn with_password<B: AsRef<[u8]>>(password: Option<B>) -> Self {
        Self {
            password: password.map(|p| p.as_ref().to_vec()),
            key_cache: KeyCache::new(),
        }
    }

    /// Returns a builder for [`ReadOptions`].
    ///
    /// # Examples
    /// ```rust
    /// use libpna::ReadOptions;
    ///
    /// let builder = ReadOptions::builder();
    /// ```
    #[inline]
    pub const fn builder() -> ReadOptionsBuilder {
        ReadOptionsBuilder::new()
    }

    /// Converts [`ReadOptions`] into a [`ReadOptionsBuilder`].
    ///
    /// # Examples
    /// ```rust
    /// use libpna::ReadOptions;
    ///
    /// let read_option = ReadOptions::builder().build();
    /// let builder = read_option.into_builder();
    /// ```
    #[inline]
    pub fn into_builder(self) -> ReadOptionsBuilder {
        self.into()
    }

    #[cfg(test)]
    pub(crate) fn cached_key_count(&self) -> usize {
        self.key_cache.len()
    }
}

/// Builder for [`ReadOptions`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ReadOptionsBuilder {
    password: Option<Vec<u8>>,
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

    /// Creates a new [`ReadOptions`].
    #[inline]
    #[must_use = "building options without using them is wasteful"]
    pub fn build(&self) -> ReadOptions {
        ReadOptions {
            password: self.password.clone(),
            key_cache: KeyCache::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn try_build_derives_key_at_build() {
        let options = WriteOptions::builder()
            .encryption(Encryption::Aes)
            .password(Some("password"))
            .try_build()
            .unwrap();
        let cipher = options.cipher.unwrap();
        assert!(!cipher.derived.phsf.is_empty());
        assert_eq!(cipher.derived.key.len(), 32);
    }

    #[test]
    fn each_build_generates_fresh_salt() {
        let mut builder = WriteOptions::builder();
        builder
            .encryption(Encryption::Aes)
            .password(Some("password"));
        let first = builder.try_build().unwrap();
        let second = builder.try_build().unwrap();
        assert_ne!(
            first.cipher.unwrap().derived.phsf,
            second.cipher.unwrap().derived.phsf,
        );
    }

    #[test]
    fn try_build_without_password_returns_error() {
        let err = WriteOptions::builder()
            .encryption(Encryption::Aes)
            .try_build()
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn try_build_with_invalid_kdf_params_returns_error() {
        let result = WriteOptions::builder()
            .encryption(Encryption::Aes)
            .hash_algorithm(HashAlgorithm::argon2id_with(Some(0), None, None))
            .password(Some("password"))
            .try_build();
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "Password was not provided.")]
    fn build_without_password_panics() {
        let _ = WriteOptions::builder().encryption(Encryption::Aes).build();
    }

    fn test_output(byte: u8) -> Output {
        Output::new(&[byte; 32]).unwrap()
    }

    #[test]
    fn key_cache_returns_inserted_key() {
        let cache = KeyCache::new();
        assert!(cache.get("phsf-a").is_none());
        cache.insert("phsf-a", test_output(1));
        assert_eq!(cache.get("phsf-a").unwrap(), test_output(1));
    }

    #[test]
    fn key_cache_clears_when_full() {
        let cache = KeyCache::new();
        for i in 0..16 {
            cache.insert(&format!("phsf-{i}"), test_output(i as u8));
        }
        assert_eq!(cache.len(), 16);
        cache.insert("phsf-16", test_output(16));
        assert_eq!(cache.len(), 1);
        assert!(cache.get("phsf-0").is_none());
        assert_eq!(cache.get("phsf-16").unwrap(), test_output(16));
    }

    #[test]
    fn read_options_clone_shares_key_cache() {
        let options = ReadOptions::with_password(Some("password"));
        let cloned = options.clone();
        cloned.key_cache.insert("phsf-a", test_output(1));
        assert_eq!(options.cached_key_count(), 1);
    }

    #[test]
    fn rebuilt_read_options_starts_with_empty_cache() {
        let options = ReadOptions::with_password(Some("password"));
        options.key_cache.insert("phsf-a", test_output(1));
        let rebuilt = options.clone().into_builder().build();
        assert_eq!(rebuilt.cached_key_count(), 0);
    }
}
