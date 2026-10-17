//! Read and write options for archive entries.

use crate::{
    cipher::{DEFAULT_SEGMENT_SIZE, SegmentSize},
    compress,
    entry::write::derive_key_material,
    error::UnknownValueError,
};
use password_hash::phc::Output;
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
        /// GCM datastream segment size; unused by CBC/CTR modes.
        pub(crate) segment_size: SegmentSize,
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
            segment_size: SegmentSize,
        ) -> Self {
            Self {
                password,
                derived,
                hash_algorithm,
                cipher_algorithm,
                mode,
                segment_size,
            }
        }
    }

    // Maximum number of derived keys retained per cache.
    //
    // Archives written after key derivation moved to WriteOptions build time
    // share a single PHSF across entries, so realistic archives hold only a
    // few distinct PHSF values. The bound prevents unbounded growth when
    // reading legacy archives that carry a distinct salt per entry.
    pub(super) const KEY_CACHE_CAP: usize = 16;

    // Cache of keys derived from PHC strings.
    //
    // Clones share the same underlying storage, so a [`ReadOptions`] and its
    // clones reuse cached keys for matching PHC strings. Correctness
    // relies on all sharers holding the same password: [`ReadOptions`] has no
    // password setter and rebuilding via a builder always starts a new cache.
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
                Compress::No => Compression::NO,
                Compress::Deflate(_) => Compression::DEFLATE,
                Compress::ZStandard(_) => Compression::ZSTANDARD,
                Compress::XZ(_) => Compression::XZ,
            }
        }

        #[inline]
        fn encryption(&self) -> Encryption {
            self.cipher()
                .map_or(Encryption::NO, |it| match it.cipher_algorithm {
                    CipherAlgorithm::Aes => Encryption::AES,
                    CipherAlgorithm::Camellia => Encryption::CAMELLIA,
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
///
/// Values without an associated constant are either reserved for future
/// PNA specification (raw value < 128) or application-specific private
/// values (raw value >= 128).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Compression(u8);

impl Compression {
    /// Do not apply any compression.
    pub const NO: Self = Self(0);
    /// Zlib format.
    pub const DEFLATE: Self = Self(1);
    /// ZStandard format.
    pub const ZSTANDARD: Self = Self(2);
    /// Xz format.
    pub const XZ: Self = Self(4);

    /// Deserializes a compression method from its u8 representation.
    ///
    /// Every byte value is a valid compression method, so this conversion
    /// never fails.
    #[inline]
    pub const fn from_byte(value: u8) -> Self {
        Self(value)
    }

    /// Serializes this compression method to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        self.0
    }

    /// Creates an application-specific private value.
    ///
    /// Returns `Some` if `value` is in the private range (`128..=255`),
    /// otherwise `None`.
    #[inline]
    pub const fn new_private(value: u8) -> Option<Self> {
        if value >= 128 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns `true` if this value is reserved for future PNA specification
    /// (unassigned and raw value < 128).
    #[inline]
    pub const fn is_reserved(self) -> bool {
        !matches!(self.0, 0..=2 | 4) && self.0 < 128
    }

    /// Returns `true` if this is an application-specific private value
    /// (raw value >= 128).
    #[inline]
    pub const fn is_private(self) -> bool {
        self.0 >= 128
    }
}

impl fmt::Debug for Compression {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NO => f.write_str("No"),
            Self::DEFLATE => f.write_str("Deflate"),
            Self::ZSTANDARD => f.write_str("ZStandard"),
            Self::XZ => f.write_str("XZ"),
            Self(v) if v < 128 => f.debug_tuple("Reserved").field(&v).finish(),
            Self(v) => f.debug_tuple("Private").field(&v).finish(),
        }
    }
}

/// Infallible; kept for backward compatibility with the former enum-based
/// API and scheduled for removal in a future release. Use
/// [`Compression::from_byte`] instead.
impl TryFrom<u8> for Compression {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_byte(value))
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

/// A compression level interpreted by the selected algorithm.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct CompressionLevel(pub(crate) CompressionLevelImpl);

impl CompressionLevel {
    pub(crate) const DEFAULT: Self = Self(CompressionLevelImpl::Default);

    /// Returns the minimum compression level of the selected algorithm.
    #[inline]
    pub const fn min() -> Self {
        Self(CompressionLevelImpl::Min)
    }

    /// Returns the maximum compression level of the selected algorithm.
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

    /// Parses a string into a [`CompressionLevel`].
    ///
    /// Accepts `"min"`, `"max"`, and `"default"` case-insensitively, or an integer level.
    ///
    /// # Errors
    ///
    /// Returns [`core::num::ParseIntError`] if `s` is neither a recognized
    /// name nor a valid integer.
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
///
/// Values without an associated constant are either reserved for future
/// PNA specification (raw value < 128) or application-specific private
/// values (raw value >= 128).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Encryption(u8);

impl Encryption {
    /// Do not apply any encryption.
    pub const NO: Self = Self(0);
    /// Aes algorithm.
    pub const AES: Self = Self(1);
    /// Camellia algorithm.
    pub const CAMELLIA: Self = Self(2);

    /// Deserializes an encryption algorithm from its u8 representation.
    ///
    /// Every byte value is a valid encryption algorithm value, so this
    /// conversion never fails.
    #[inline]
    pub const fn from_byte(value: u8) -> Self {
        Self(value)
    }

    /// Serializes this encryption algorithm to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        self.0
    }

    /// Creates an application-specific private value.
    ///
    /// Returns `Some` if `value` is in the private range (`128..=255`),
    /// otherwise `None`.
    #[inline]
    pub const fn new_private(value: u8) -> Option<Self> {
        if value >= 128 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns `true` if this value is reserved for future PNA specification
    /// (unassigned and raw value < 128).
    #[inline]
    pub const fn is_reserved(self) -> bool {
        !matches!(self.0, 0..=2) && self.0 < 128
    }

    /// Returns `true` if this is an application-specific private value
    /// (raw value >= 128).
    #[inline]
    pub const fn is_private(self) -> bool {
        self.0 >= 128
    }
}

impl fmt::Debug for Encryption {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NO => f.write_str("No"),
            Self::AES => f.write_str("Aes"),
            Self::CAMELLIA => f.write_str("Camellia"),
            Self(v) if v < 128 => f.debug_tuple("Reserved").field(&v).finish(),
            Self(v) => f.debug_tuple("Private").field(&v).finish(),
        }
    }
}

/// Infallible; kept for backward compatibility with the former enum-based
/// API and scheduled for removal in a future release. Use
/// [`Encryption::from_byte`] instead.
impl TryFrom<u8> for Encryption {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_byte(value))
    }
}

/// Cipher mode of encryption algorithm.
///
/// Values without an associated constant are either reserved for future
/// PNA specification (raw value < 128) or application-specific private
/// values (raw value >= 128).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CipherMode(u8);

impl CipherMode {
    /// Cipher Block Chaining mode.
    pub const CBC: Self = Self(0);
    /// Counter mode.
    pub const CTR: Self = Self(1);
    /// Galois/Counter mode (AEAD, STREAM-based).
    ///
    /// Authenticates the encrypted data and detects tampering, unlike CBC and CTR.
    pub const GCM: Self = Self(2);

    /// Deserializes a cipher mode from its u8 representation.
    ///
    /// Every byte value is a valid cipher mode value, so this conversion
    /// never fails.
    #[inline]
    pub const fn from_byte(value: u8) -> Self {
        Self(value)
    }

    /// Serializes this cipher mode to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        self.0
    }

    /// Creates an application-specific private value.
    ///
    /// Returns `Some` if `value` is in the private range (`128..=255`),
    /// otherwise `None`.
    #[inline]
    pub const fn new_private(value: u8) -> Option<Self> {
        if value >= 128 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns `true` if this value is reserved for future PNA specification
    /// (unassigned and raw value < 128).
    #[inline]
    pub const fn is_reserved(self) -> bool {
        !matches!(self.0, 0..=2) && self.0 < 128
    }

    /// Returns `true` if this is an application-specific private value
    /// (raw value >= 128).
    #[inline]
    pub const fn is_private(self) -> bool {
        self.0 >= 128
    }

    /// Returns `true` if an entry's header can be rewritten without making its
    /// encrypted data unreadable.
    ///
    /// Returns `true` for [`CipherMode::CBC`] and [`CipherMode::CTR`], and
    /// `false` for [`CipherMode::GCM`] and unsupported modes. GCM binds the
    /// encrypted data to the entry header, including its name.
    #[inline]
    pub const fn allows_header_rewrite(self) -> bool {
        matches!(self, Self::CBC | Self::CTR)
    }
}

impl fmt::Debug for CipherMode {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::CBC => f.write_str("CBC"),
            Self::CTR => f.write_str("CTR"),
            Self::GCM => f.write_str("GCM"),
            Self(v) if v < 128 => f.debug_tuple("Reserved").field(&v).finish(),
            Self(v) => f.debug_tuple("Private").field(&v).finish(),
        }
    }
}

/// Infallible; kept for backward compatibility with the former enum-based
/// API and scheduled for removal in a future release. Use
/// [`CipherMode::from_byte`] instead.
impl TryFrom<u8> for CipherMode {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_byte(value))
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
    /// Creates PBKDF2-SHA256 parameters with the default iteration count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{Encryption, HashAlgorithm, WriteOptions};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::AES)
    ///     .hash_algorithm(HashAlgorithm::pbkdf2_sha256())
    ///     .password(Some("password"))
    ///     .build();
    /// ```
    #[inline]
    pub const fn pbkdf2_sha256() -> Self {
        Self::pbkdf2_sha256_with(None)
    }

    /// Creates PBKDF2-SHA256 parameters with the specified iteration count.
    ///
    /// `None` uses the default iteration count.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{Encryption, HashAlgorithm, WriteOptions};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::AES)
    ///     .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(100_000)))
    ///     .password(Some("password"))
    ///     .build();
    /// ```
    #[inline]
    pub const fn pbkdf2_sha256_with(rounds: Option<u32>) -> Self {
        Self(HashAlgorithmParams::Pbkdf2Sha256 { rounds })
    }

    /// Creates Argon2id parameters with the default costs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{Encryption, HashAlgorithm, WriteOptions};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::AES)
    ///     .hash_algorithm(HashAlgorithm::argon2id())
    ///     .password(Some("secure_password"))
    ///     .build();
    /// ```
    #[inline]
    pub const fn argon2id() -> Self {
        Self::argon2id_with(None, None, None)
    }

    /// Creates Argon2id parameters with the specified costs.
    ///
    /// `time_cost` is the number of iterations, `memory_cost` is the memory
    /// usage in KiB, and `parallelism_cost` is the number of lanes.
    /// `None` uses the default value for the corresponding parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{Encryption, HashAlgorithm, WriteOptions};
    ///
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::AES)
    ///     .hash_algorithm(HashAlgorithm::argon2id_with(
    ///         Some(4),     // time_cost: 4 iterations
    ///         Some(65536), // memory_cost: 64 MiB
    ///         Some(2),     // parallelism: 2 lanes
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
/// Each value determines how the entry's data should be interpreted
/// and how the entry should be extracted to the filesystem. Values
/// without an associated constant are either reserved for future PNA
/// specification (raw value < 128) or application-specific private
/// values (raw value >= 128).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DataKind(u8);

impl DataKind {
    /// Regular file. Entry data contains the file contents.
    pub const FILE: Self = Self(0);
    /// Directory. Entry has no data content.
    pub const DIRECTORY: Self = Self(1);
    /// Symbolic link. Entry data contains the UTF-8 encoded link target path.
    pub const SYMBOLIC_LINK: Self = Self(2);
    /// Hard link. Entry data contains the UTF-8 encoded path of the target
    /// entry within the same archive.
    pub const HARD_LINK: Self = Self(3);

    /// Deserializes a data kind from its u8 representation.
    ///
    /// Every byte value is a valid data kind, so this conversion never fails.
    #[inline]
    pub const fn from_byte(value: u8) -> Self {
        Self(value)
    }

    /// Serializes this data kind to its u8 representation.
    #[inline]
    pub const fn to_byte(self) -> u8 {
        self.0
    }

    /// Creates an application-specific private value.
    ///
    /// Returns `Some` if `value` is in the private range (`128..=255`),
    /// otherwise `None`.
    #[inline]
    pub const fn new_private(value: u8) -> Option<Self> {
        if value >= 128 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns `true` if this value is reserved for future PNA specification
    /// (unassigned and raw value < 128).
    #[inline]
    pub const fn is_reserved(self) -> bool {
        !matches!(self.0, 0..=3) && self.0 < 128
    }

    /// Returns `true` if this is an application-specific private value
    /// (raw value >= 128).
    #[inline]
    pub const fn is_private(self) -> bool {
        self.0 >= 128
    }
}

impl fmt::Debug for DataKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::FILE => f.write_str("File"),
            Self::DIRECTORY => f.write_str("Directory"),
            Self::SYMBOLIC_LINK => f.write_str("SymbolicLink"),
            Self::HARD_LINK => f.write_str("HardLink"),
            Self(v) if v < 128 => f.debug_tuple("Reserved").field(&v).finish(),
            Self(v) => f.debug_tuple("Private").field(&v).finish(),
        }
    }
}

/// Infallible; kept for backward compatibility with the former enum-based
/// API and scheduled for removal in a future release. Use
/// [`DataKind::from_byte`] instead.
impl TryFrom<u8> for DataKind {
    type Error = UnknownValueError;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self::from_byte(value))
    }
}

/// Options for compressing and encrypting archive entries.
///
/// Data is compressed before it is encrypted. Use [`WriteOptions::builder`]
/// to configure the options, or [`WriteOptions::store`] to disable both.
///
/// When encryption is enabled, the password-derived key and KDF salt are
/// shared by every entry written with these options, including clones.
/// Build fresh options for each archive to use an independent salt and key.
/// Each entry receives fresh random encryption material: an IV for CBC/CTR,
/// or a salt and nonce prefix for GCM.
///
/// Use [`ReadOptions`] with the password to read encrypted entries. Compression
/// and cipher settings are stored in the archive.
///
/// # Examples
///
/// ```
/// use libpna::{Compression, Encryption, WriteOptions};
///
/// let options = WriteOptions::builder()
///     .compression(Compression::ZSTANDARD)
///     .encryption(Encryption::AES)
///     .password(Some("password"))
///     .build();
/// ```
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
    /// use libpna::{FileEntryBuilder, WriteOptions};
    ///
    /// FileEntryBuilder::new_with_options("example.txt".into(), WriteOptions::store()).unwrap();
    /// ```
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
    ///
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

const DEFAULT_CIPHER_MODE: CipherMode = CipherMode::GCM;
const DEFAULT_HASH_ALGORITHM: HashAlgorithm = HashAlgorithm::argon2id();

/// Builder for [`WriteOptions`].
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WriteOptionsBuilder {
    compression: Compression,
    compression_level: CompressionLevel,
    encryption: Encryption,
    cipher_mode: Option<CipherMode>,
    hash_algorithm: Option<HashAlgorithm>,
    password: Option<Vec<u8>>,
    segment_size: Option<u32>,
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
            Compress::No => (Compression::NO, CompressionLevel::DEFAULT),
            Compress::Deflate(level) => (Compression::DEFLATE, level.into()),
            Compress::ZStandard(level) => (Compression::ZSTANDARD, level.into()),
            Compress::XZ(level) => (Compression::XZ, level.into()),
        };
        Self {
            compression,
            compression_level,
            encryption: value.encryption(),
            // Unencrypted options carry nothing to inherit for these three, so
            // leave the choice open rather than adopting the values they report
            // — those are fallbacks, not decisions the caller made.
            cipher_mode: value.cipher().map(|it| it.mode),
            hash_algorithm: value.cipher().map(|it| it.hash_algorithm),
            password: value.password().map(|p| p.to_vec()),
            segment_size: value.cipher().map(|it| it.segment_size.get()),
        }
    }
}

impl WriteOptionsBuilder {
    const fn new() -> Self {
        Self {
            compression: Compression::NO,
            compression_level: CompressionLevel::DEFAULT,
            encryption: Encryption::NO,
            cipher_mode: None,
            hash_algorithm: None,
            password: None,
            segment_size: None,
        }
    }

    /// Sets the compression method.
    #[inline]
    pub fn compression(&mut self, compression: Compression) -> &mut Self {
        self.compression = compression;
        self
    }

    /// Sets the compression level.
    #[inline]
    pub fn compression_level(&mut self, compression_level: CompressionLevel) -> &mut Self {
        self.compression_level = compression_level;
        self
    }

    /// Sets the encryption algorithm.
    #[inline]
    pub fn encryption(&mut self, encryption: Encryption) -> &mut Self {
        self.encryption = encryption;
        self
    }

    /// Sets the cipher mode.
    ///
    /// The default is [`CipherMode::GCM`].
    #[inline]
    pub fn cipher_mode(&mut self, cipher_mode: CipherMode) -> &mut Self {
        self.cipher_mode = Some(cipher_mode);
        self
    }

    /// Sets the password hash algorithm.
    ///
    /// The default is [`HashAlgorithm::argon2id`].
    #[inline]
    pub fn hash_algorithm(&mut self, algorithm: HashAlgorithm) -> &mut Self {
        self.hash_algorithm = Some(algorithm);
        self
    }

    /// Sets the password.
    ///
    /// Accepts both UTF-8 strings and arbitrary byte slices.
    ///
    /// # Examples
    ///
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

    /// Sets the GCM datastream segment size in bytes.
    #[cfg(test)]
    #[inline]
    pub(crate) fn segment_size(&mut self, size: u32) -> &mut Self {
        self.segment_size = Some(size);
        self
    }

    /// Creates a new [`WriteOptions`] from this builder, deriving the encryption
    /// key when encryption is enabled.
    ///
    /// Generates a fresh KDF salt when encryption is enabled. The resulting
    /// options share the salt and derived key as described in [`WriteOptions`].
    ///
    /// # Errors
    ///
    /// - Encryption is enabled but no password was provided.
    /// - Key derivation or random salt generation fails, including invalid KDF parameters.
    /// - An unsupported encryption or compression method was specified.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use libpna::{Encryption, WriteOptions};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// let opts = WriteOptions::builder()
    ///     .encryption(Encryption::AES)
    ///     .password(Some("password"))
    ///     .try_build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use = "building options without using them is wasteful"]
    pub fn try_build(&self) -> io::Result<WriteOptions> {
        let cipher = if self.encryption != Encryption::NO {
            let cipher_algorithm = match self.encryption {
                Encryption::AES => CipherAlgorithm::Aes,
                Encryption::CAMELLIA => CipherAlgorithm::Camellia,
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
            let requested = self.segment_size.unwrap_or(DEFAULT_SEGMENT_SIZE);
            let segment_size = SegmentSize::new(requested).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("segment size out of range: {requested}"),
                )
            })?;
            let hash_algorithm = self.hash_algorithm.unwrap_or(DEFAULT_HASH_ALGORITHM);
            let derived = derive_key_material(cipher_algorithm, hash_algorithm, password)?;
            Some(Cipher::new(
                password.into(),
                derived,
                hash_algorithm,
                cipher_algorithm,
                self.cipher_mode.unwrap_or(DEFAULT_CIPHER_MODE),
                segment_size,
            ))
        } else {
            None
        };
        Ok(WriteOptions {
            compress: match self.compression {
                Compression::NO => Compress::No,
                Compression::DEFLATE => Compress::Deflate(self.compression_level.into()),
                Compression::ZSTANDARD => Compress::ZStandard(self.compression_level.into()),
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

    /// Builds the configured [`WriteOptions`].
    ///
    /// # Panics
    ///
    /// Panics if [`try_build`](Self::try_build) returns an error.
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
/// Derived encryption keys are cached and reused when reading entries with
/// the same password hash parameters and salt. Clones share the cache;
/// rebuilding through [`ReadOptions::into_builder`] starts with an empty cache.
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
    ///
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
    ///
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
    ///
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
            .encryption(Encryption::AES)
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
            .encryption(Encryption::AES)
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
            .encryption(Encryption::AES)
            .try_build()
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn try_build_with_out_of_range_segment_size_returns_error() {
        for size in [0, crate::cipher::MAX_SEGMENT_SIZE + 1] {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::AES)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
                .password(Some("password"));
            builder.segment_size(size);
            let err = builder.try_build().unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn try_build_accepts_boundary_segment_sizes() {
        for size in [1, crate::cipher::MAX_SEGMENT_SIZE] {
            let mut builder = WriteOptions::builder();
            builder
                .encryption(Encryption::AES)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
                .password(Some("password"));
            builder.segment_size(size);
            let options = builder.try_build().unwrap();
            assert_eq!(options.cipher().unwrap().segment_size.get(), size);
        }
    }

    #[test]
    fn try_build_with_invalid_kdf_params_returns_error() {
        let result = WriteOptions::builder()
            .encryption(Encryption::AES)
            .hash_algorithm(HashAlgorithm::argon2id_with(Some(0), None, None))
            .password(Some("password"))
            .try_build();
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "Password was not provided.")]
    fn build_without_password_panics() {
        let _ = WriteOptions::builder().encryption(Encryption::AES).build();
    }

    #[test]
    fn default_cipher_mode_is_gcm() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
            .password(Some("password"))
            .try_build()
            .unwrap();
        assert_eq!(options.cipher_mode(), CipherMode::GCM);
        assert_eq!(
            options.hash_algorithm(),
            HashAlgorithm::pbkdf2_sha256_with(Some(1000))
        );
    }

    #[test]
    fn into_builder_of_unencrypted_options_keeps_the_default_cipher_mode() {
        assert_eq!(WriteOptions::store().cipher_mode(), CipherMode::CTR);
        let options = WriteOptions::store()
            .into_builder()
            .encryption(Encryption::AES)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
            .password(Some("password"))
            .try_build()
            .unwrap();
        assert_eq!(options.cipher_mode(), CipherMode::GCM);
    }

    #[test]
    fn into_builder_preserves_an_explicit_cipher_mode() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .cipher_mode(CipherMode::CTR)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1000)))
            .password(Some("password"))
            .try_build()
            .unwrap();
        let rebuilt = options.into_builder().try_build().unwrap();
        assert_eq!(rebuilt.cipher_mode(), CipherMode::CTR);
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
    fn key_cache_stays_bounded_and_retains_newly_inserted_keys() {
        let cache = KeyCache::new();
        for i in 0..=private::KEY_CACHE_CAP * 3 {
            let phsf = format!("phsf-{i}");
            let key = test_output(i as u8);
            cache.insert(&phsf, key);
            assert!(cache.len() <= private::KEY_CACHE_CAP);
            assert_eq!(cache.get(&phsf), Some(key));
        }
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

    #[test]
    fn data_kind_round_trips_all_byte_values() {
        for v in 0..=u8::MAX {
            assert_eq!(DataKind::from_byte(v).to_byte(), v);
        }
    }

    #[test]
    fn data_kind_known_constants_map_to_spec_bytes() {
        assert_eq!(DataKind::FILE.to_byte(), 0);
        assert_eq!(DataKind::DIRECTORY.to_byte(), 1);
        assert_eq!(DataKind::SYMBOLIC_LINK.to_byte(), 2);
        assert_eq!(DataKind::HARD_LINK.to_byte(), 3);
    }

    #[test]
    fn data_kind_new_private_boundary() {
        assert_eq!(DataKind::new_private(0), None);
        assert_eq!(DataKind::new_private(127), None);
        assert_eq!(DataKind::new_private(128), Some(DataKind::from_byte(128)));
        assert_eq!(DataKind::new_private(255), Some(DataKind::from_byte(255)));
    }

    #[test]
    fn data_kind_predicates() {
        assert!(!DataKind::FILE.is_reserved());
        assert!(!DataKind::FILE.is_private());
        assert!(!DataKind::HARD_LINK.is_reserved());
        assert!(DataKind::from_byte(4).is_reserved());
        assert!(DataKind::from_byte(127).is_reserved());
        assert!(!DataKind::from_byte(127).is_private());
        assert!(!DataKind::from_byte(128).is_reserved());
        assert!(DataKind::from_byte(128).is_private());
        assert!(DataKind::from_byte(255).is_private());
    }

    #[test]
    fn data_kind_debug_matches_former_enum_output() {
        assert_eq!(format!("{:?}", DataKind::FILE), "File");
        assert_eq!(format!("{:?}", DataKind::DIRECTORY), "Directory");
        assert_eq!(format!("{:?}", DataKind::SYMBOLIC_LINK), "SymbolicLink");
        assert_eq!(format!("{:?}", DataKind::HARD_LINK), "HardLink");
        assert_eq!(format!("{:?}", DataKind::from_byte(5)), "Reserved(5)");
        assert_eq!(format!("{:?}", DataKind::from_byte(200)), "Private(200)");
    }

    #[test]
    fn data_kind_try_from_is_infallible_and_matches_from_byte() {
        for v in 0..=u8::MAX {
            assert_eq!(
                <DataKind as TryFrom<u8>>::try_from(v).unwrap(),
                DataKind::from_byte(v)
            );
        }
    }

    #[test]
    fn compression_round_trips_all_byte_values() {
        for v in 0..=u8::MAX {
            assert_eq!(Compression::from_byte(v).to_byte(), v);
        }
    }

    #[test]
    fn compression_known_constants_map_to_spec_bytes() {
        assert_eq!(Compression::NO.to_byte(), 0);
        assert_eq!(Compression::DEFLATE.to_byte(), 1);
        assert_eq!(Compression::ZSTANDARD.to_byte(), 2);
        assert_eq!(Compression::XZ.to_byte(), 4);
    }

    #[test]
    fn compression_new_private_boundary() {
        assert_eq!(Compression::new_private(0), None);
        assert_eq!(Compression::new_private(127), None);
        assert_eq!(
            Compression::new_private(128),
            Some(Compression::from_byte(128))
        );
        assert_eq!(
            Compression::new_private(255),
            Some(Compression::from_byte(255))
        );
    }

    #[test]
    fn compression_predicates() {
        assert!(!Compression::NO.is_reserved());
        assert!(!Compression::XZ.is_reserved());
        assert!(!Compression::XZ.is_private());
        assert!(Compression::from_byte(3).is_reserved());
        assert!(Compression::from_byte(127).is_reserved());
        assert!(!Compression::from_byte(127).is_private());
        assert!(!Compression::from_byte(128).is_reserved());
        assert!(Compression::from_byte(128).is_private());
        assert!(Compression::from_byte(255).is_private());
    }

    #[test]
    fn compression_debug_matches_former_enum_output() {
        assert_eq!(format!("{:?}", Compression::NO), "No");
        assert_eq!(format!("{:?}", Compression::DEFLATE), "Deflate");
        assert_eq!(format!("{:?}", Compression::ZSTANDARD), "ZStandard");
        assert_eq!(format!("{:?}", Compression::XZ), "XZ");
        assert_eq!(format!("{:?}", Compression::from_byte(3)), "Reserved(3)");
        assert_eq!(format!("{:?}", Compression::from_byte(200)), "Private(200)");
    }

    #[test]
    fn compression_try_from_is_infallible_and_matches_from_byte() {
        for v in 0..=u8::MAX {
            assert_eq!(
                <Compression as TryFrom<u8>>::try_from(v).unwrap(),
                Compression::from_byte(v)
            );
        }
    }

    #[test]
    fn try_build_with_unknown_compression_returns_unsupported() {
        let err = WriteOptions::builder()
            .compression(Compression::from_byte(5))
            .try_build()
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn encryption_round_trips_all_byte_values() {
        for v in 0..=u8::MAX {
            assert_eq!(Encryption::from_byte(v).to_byte(), v);
        }
    }

    #[test]
    fn encryption_known_constants_map_to_spec_bytes() {
        assert_eq!(Encryption::NO.to_byte(), 0);
        assert_eq!(Encryption::AES.to_byte(), 1);
        assert_eq!(Encryption::CAMELLIA.to_byte(), 2);
    }

    #[test]
    fn encryption_new_private_boundary() {
        assert_eq!(Encryption::new_private(0), None);
        assert_eq!(Encryption::new_private(127), None);
        assert_eq!(
            Encryption::new_private(128),
            Some(Encryption::from_byte(128))
        );
        assert_eq!(
            Encryption::new_private(255),
            Some(Encryption::from_byte(255))
        );
    }

    #[test]
    fn encryption_predicates() {
        assert!(!Encryption::NO.is_reserved());
        assert!(!Encryption::CAMELLIA.is_reserved());
        assert!(!Encryption::CAMELLIA.is_private());
        assert!(Encryption::from_byte(3).is_reserved());
        assert!(Encryption::from_byte(127).is_reserved());
        assert!(!Encryption::from_byte(127).is_private());
        assert!(!Encryption::from_byte(128).is_reserved());
        assert!(Encryption::from_byte(128).is_private());
        assert!(Encryption::from_byte(255).is_private());
    }

    #[test]
    fn encryption_debug_matches_former_enum_output() {
        assert_eq!(format!("{:?}", Encryption::NO), "No");
        assert_eq!(format!("{:?}", Encryption::AES), "Aes");
        assert_eq!(format!("{:?}", Encryption::CAMELLIA), "Camellia");
        assert_eq!(format!("{:?}", Encryption::from_byte(3)), "Reserved(3)");
        assert_eq!(format!("{:?}", Encryption::from_byte(200)), "Private(200)");
    }

    #[test]
    fn encryption_try_from_is_infallible_and_matches_from_byte() {
        for v in 0..=u8::MAX {
            assert_eq!(
                <Encryption as TryFrom<u8>>::try_from(v).unwrap(),
                Encryption::from_byte(v)
            );
        }
    }

    #[test]
    fn try_build_with_unknown_encryption_returns_unsupported() {
        let err = WriteOptions::builder()
            .encryption(Encryption::from_byte(5))
            .password(Some("password"))
            .try_build()
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    #[test]
    fn cipher_mode_round_trips_all_byte_values() {
        for v in 0..=u8::MAX {
            assert_eq!(CipherMode::from_byte(v).to_byte(), v);
        }
    }

    #[test]
    fn cipher_mode_known_constants_map_to_spec_bytes() {
        assert_eq!(CipherMode::CBC.to_byte(), 0);
        assert_eq!(CipherMode::CTR.to_byte(), 1);
        assert_eq!(CipherMode::GCM.to_byte(), 2);
        assert_eq!(CipherMode::from_byte(2), CipherMode::GCM);
    }

    #[test]
    fn cipher_mode_new_private_boundary() {
        assert_eq!(CipherMode::new_private(0), None);
        assert_eq!(CipherMode::new_private(127), None);
        assert_eq!(
            CipherMode::new_private(128),
            Some(CipherMode::from_byte(128))
        );
        assert_eq!(
            CipherMode::new_private(255),
            Some(CipherMode::from_byte(255))
        );
    }

    #[test]
    fn cipher_mode_predicates() {
        assert!(!CipherMode::CBC.is_reserved());
        assert!(!CipherMode::CTR.is_reserved());
        assert!(!CipherMode::CTR.is_private());
        assert!(!CipherMode::GCM.is_reserved());
        assert!(CipherMode::from_byte(3).is_reserved());
        assert!(CipherMode::from_byte(127).is_reserved());
        assert!(!CipherMode::from_byte(127).is_private());
        assert!(!CipherMode::from_byte(128).is_reserved());
        assert!(CipherMode::from_byte(128).is_private());
        assert!(CipherMode::from_byte(255).is_private());
    }

    #[test]
    fn allows_header_rewrite_only_for_cbc_and_ctr() {
        assert!(CipherMode::CBC.allows_header_rewrite());
        assert!(CipherMode::CTR.allows_header_rewrite());
        assert!(!CipherMode::GCM.allows_header_rewrite());
    }

    #[test]
    fn allows_header_rewrite_is_false_for_unassigned_cipher_modes() {
        for byte in [3u8, 63, 128, 255] {
            assert!(
                !CipherMode::from_byte(byte).allows_header_rewrite(),
                "byte={byte}"
            );
        }
    }

    #[test]
    fn cipher_mode_debug_matches_former_enum_output() {
        assert_eq!(format!("{:?}", CipherMode::CBC), "CBC");
        assert_eq!(format!("{:?}", CipherMode::CTR), "CTR");
        assert_eq!(format!("{:?}", CipherMode::GCM), "GCM");
        assert_eq!(format!("{:?}", CipherMode::from_byte(3)), "Reserved(3)");
        assert_eq!(format!("{:?}", CipherMode::from_byte(200)), "Private(200)");
    }

    #[test]
    fn cipher_mode_try_from_is_infallible_and_matches_from_byte() {
        for v in 0..=u8::MAX {
            assert_eq!(
                <CipherMode as TryFrom<u8>>::try_from(v).unwrap(),
                CipherMode::from_byte(v)
            );
        }
    }
}
