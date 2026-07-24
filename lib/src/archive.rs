//! Archive reading and writing for PNA files.

mod header;
mod read;
mod write;

use crate::{
    chunk::{ChunkStreamWriter, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
};
use core::num::NonZeroU32;
pub use header::*;
use std::io::prelude::*;
pub(crate) use {read::*, write::*};

/// Provides read and write access to a PNA file.
///
/// An instance of an [`Archive`] can be read and/or written.
///
/// The [`Archive`] struct provides two main modes of operation:
/// - Read mode: Allows reading entries from an existing PNA file
/// - Write mode: Enables creating new entries and writing data to the archive
///
/// The archive supports various features including:
/// - Multiple compression algorithms
/// - Encryption options
/// - Solid and non-solid modes
/// - Chunk-based storage
///
/// # Examples
/// Creates a new PNA file and adds an entry to it.
/// ```no_run
/// # use libpna::{Archive, FileEntryBuilder, WriteOptions};
/// # use std::fs::File;
/// # use std::io::{self, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let file = File::create("foo.pna")?;
/// let mut archive = Archive::write_header(file)?;
/// let mut entry_builder =
///     FileEntryBuilder::new_with_options("bar.txt".into(), WriteOptions::builder().build())?;
/// entry_builder.write_all(b"content")?;
/// let entry = entry_builder.build()?;
/// archive.add_entry(entry)?;
/// archive.finalize()?;
/// #     Ok(())
/// # }
/// ```
///
/// Reads the entries of a PNA file.
/// ```no_run
/// # use libpna::{Archive, ReadOptions};
/// # use std::fs::File;
/// # use std::io::{self, copy, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let file = File::open("foo.pna")?;
/// let mut archive = Archive::read_header(file)?;
/// for entry in archive.entries().skip_solid() {
///     let entry = entry?;
///     let mut file = File::create(entry.header().path().as_path())?;
///     let mut reader = entry.reader(ReadOptions::builder().build())?;
///     copy(&mut reader, &mut file)?;
/// }
/// #     Ok(())
/// # }
/// ```
pub struct Archive<T> {
    inner: T,
    header: ArchiveHeader,
    max_chunk_size: Option<NonZeroU32>,
    // following fields are only use in reader mode
    next_archive: bool,
    buf: Vec<RawChunk>,
}

impl<T> Archive<T> {
    const fn new(inner: T, header: ArchiveHeader) -> Self {
        Self::with_buffer(inner, header, Vec::new())
    }

    const fn with_buffer(inner: T, header: ArchiveHeader, buf: Vec<RawChunk>) -> Self {
        Self {
            inner,
            header,
            max_chunk_size: None,
            next_archive: false,
            buf,
        }
    }

    /// Sets the maximum chunk size limit.
    ///
    /// When set, this limit affects both reading and writing:
    /// - **Reading**: Chunks larger than this size will be rejected with an error,
    ///   protecting against maliciously crafted archives with extremely large chunks.
    /// - **Writing**: Data written via [`write_file()`](Archive::write_file) will be
    ///   split into chunks no larger than this size.
    ///
    /// **Note**: This setting only affects the streaming write path
    /// ([`write_file()`](Archive::write_file)). Pre-built entries added via
    /// [`add_entry()`](Archive::add_entry) use their own chunk size configured
    /// through [`FileEntryBuilder::max_chunk_size()`](crate::FileEntryBuilder::max_chunk_size).
    ///
    #[inline]
    pub fn set_max_chunk_size(&mut self, size: NonZeroU32) {
        self.max_chunk_size = Some(size);
    }

    /// Returns `true` if an [`ANXT`] chunk has been encountered during reading.
    ///
    /// [`ANXT`]: crate::chunk::ChunkType::ANXT
    #[inline]
    pub const fn has_next_archive(&self) -> bool {
        self.next_archive
    }

    /// Consumes the archive and returns the underlying reader or writer.
    ///
    /// # Warning
    ///
    /// This method does not finalize the archive. If you are writing to an
    /// archive, call [`Archive::finalize`] first to ensure the end-of-archive
    /// marker is written. Using `into_inner` on a writer without finalizing
    /// leaves the archive incomplete.
    ///
    /// # Examples
    ///
    /// For normal archive completion, prefer [`Archive::finalize`] which writes
    /// the end-of-archive marker and returns the inner writer:
    ///
    /// ```
    /// # use libpna::Archive;
    /// # use std::io;
    /// # fn main() -> io::Result<()> {
    /// let archive = Archive::write_header(Vec::new())?;
    /// let writer = archive.finalize()?; // Preferred: archive is properly closed
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Use `into_inner` when you need to abandon an archive or access the
    /// underlying reader:
    ///
    /// ```
    /// # use libpna::Archive;
    /// # use std::io;
    /// # fn main() -> io::Result<()> {
    /// let file = std::io::Cursor::new(include_bytes!("../../resources/test/empty.pna").to_vec());
    /// let archive = Archive::read_header(file)?;
    /// let _reader = archive.into_inner(); // Safe for readers
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "call `finalize` instead if you don't need the inner value"]
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

/// Provides write access to solid mode PNA files.
///
/// In solid mode, all entries are compressed together as a single unit,
/// which typically results in better compression ratios compared to
/// non-solid mode. However, this means that individual entries cannot
/// be accessed randomly - they must be read sequentially.
///
/// Key features of solid mode:
/// - Improved compression ratio
/// - Sequential access only
/// - Single compression/encryption context for all entries
///
/// # Examples
/// Creates a new solid mode PNA file and adds an entry to it.
/// ```no_run
/// use libpna::{Archive, FileEntryBuilder, WriteOptions};
/// use std::fs::File;
/// # use std::io::{self, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let option = WriteOptions::builder().build();
/// let file = File::create("foo.pna")?;
/// let mut archive = Archive::write_solid_header(file, option)?;
/// let mut entry_builder = FileEntryBuilder::new("bar.txt".into())?;
/// entry_builder.write_all(b"content")?;
/// let entry = entry_builder.build()?;
/// archive.add_entry(entry)?;
/// archive.finalize()?;
/// #     Ok(())
/// # }
/// ```
pub struct SolidArchive<T: Write> {
    archive_header: ArchiveHeader,
    inner: CompressionWriter<CipherWriter<ChunkStreamWriter<T>>>,
    max_chunk_size: Option<NonZeroU32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Duration, entry::*};
    use std::io::{self, Cursor};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn store_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder().compression(Compression::NO).build(),
        )
        .unwrap()
    }

    #[test]
    fn deflate_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder()
                .compression(Compression::DEFLATE)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn zstd_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn xz_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder().compression(Compression::XZ).build(),
        )
        .unwrap();
    }

    #[test]
    fn store_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::NO)
                .encryption(Encryption::AES)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_aes_ctr_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .encryption(Encryption::AES)
                .cipher_mode(CipherMode::CTR)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .encryption(Encryption::AES)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_camellia_ctr_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .encryption(Encryption::CAMELLIA)
                .cipher_mode(CipherMode::CTR)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_camellia_cbc_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::ZSTANDARD)
                .encryption(Encryption::CAMELLIA)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn xz_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::XZ)
                .encryption(Encryption::AES)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn xz_with_camellia_cbc_archive() {
        archive(
            b"plain text",
            WriteOptions::builder()
                .compression(Compression::XZ)
                .encryption(Encryption::CAMELLIA)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap()
    }

    /// Precondition: encryption-enabled WriteOptions built once.
    /// Action: write two entries with the same options and read them back.
    /// Expectation: entries share the PHSF (derived key), have distinct
    /// ciphertext for identical plaintext (unique IVs), and both decrypt.
    #[test]
    fn entries_share_derived_key_with_unique_iv() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .password(Some("password"))
            .build();
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        for name in ["test/first", "test/second"] {
            writer
                .add_entry({
                    let mut builder =
                        FileEntryBuilder::new_with_options(name.into(), &options).unwrap();
                    builder.write_all(b"same plaintext").unwrap();
                    builder.build().unwrap()
                })
                .unwrap();
        }
        let archived = writer.finalize().unwrap();

        let mut reader = Archive::read_header(archived.as_slice()).unwrap();
        let entries = reader
            .entries()
            .skip_solid()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].phsf.is_some());
        assert_eq!(entries[0].phsf, entries[1].phsf);
        assert_ne!(entries[0].data, entries[1].data);
        for entry in &entries {
            let mut data_reader = entry
                .reader(ReadOptions::with_password(Some("password")))
                .unwrap();
            let mut dist = Vec::new();
            io::copy(&mut data_reader, &mut dist).unwrap();
            assert_eq!(dist.as_slice(), b"same plaintext");
        }
    }

    /// Precondition: encryption-enabled WriteOptions built once, reused for
    /// multiple entries so every entry carries the same PHSF.
    /// Action: read all entries back with a single, shared ReadOptions.
    /// Expectation: every entry decrypts correctly and the key derivation
    /// function ran only once, since the derived key was cached.
    #[test]
    fn read_options_derives_key_once_per_archive() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .password(Some("password"))
            .try_build()
            .unwrap();
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        for name in ["test/first", "test/second", "test/third"] {
            writer
                .add_entry({
                    let mut builder =
                        FileEntryBuilder::new_with_options(name.into(), &options).unwrap();
                    builder.write_all(b"some text").unwrap();
                    builder.build().unwrap()
                })
                .unwrap();
        }
        let archived = writer.finalize().unwrap();

        let read_options = ReadOptions::with_password(Some("password"));
        let mut reader = Archive::read_header(archived.as_slice()).unwrap();
        let entries = reader
            .entries()
            .skip_solid()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(entries.len(), 3);
        for entry in &entries {
            let mut data_reader = entry.reader(&read_options).unwrap();
            let mut dist = Vec::new();
            io::copy(&mut data_reader, &mut dist).unwrap();
            assert_eq!(dist.as_slice(), b"some text");
        }
        assert_eq!(read_options.cached_key_count(), 1);
    }

    /// Precondition: entries with distinct PHSF values (as in archives
    /// written before key derivation moved to WriteOptions build time, where
    /// each entry gets its own salt).
    /// Action: read all entries back with a single, shared ReadOptions.
    /// Expectation: every entry decrypts correctly and the cache holds one
    /// entry per distinct PHSF.
    #[test]
    fn read_options_cache_handles_distinct_phsf_entries() {
        let mut options_builder = WriteOptions::builder();
        options_builder
            .encryption(Encryption::AES)
            .password(Some("password"));
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        for name in ["test/first", "test/second"] {
            let options = options_builder.try_build().unwrap();
            writer
                .add_entry({
                    let mut builder =
                        FileEntryBuilder::new_with_options(name.into(), &options).unwrap();
                    builder.write_all(b"some text").unwrap();
                    builder.build().unwrap()
                })
                .unwrap();
        }
        let archived = writer.finalize().unwrap();

        let read_options = ReadOptions::with_password(Some("password"));
        let mut reader = Archive::read_header(archived.as_slice()).unwrap();
        let entries = reader
            .entries()
            .skip_solid()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_ne!(entries[0].phsf, entries[1].phsf);
        for entry in &entries {
            let mut data_reader = entry.reader(&read_options).unwrap();
            let mut dist = Vec::new();
            io::copy(&mut data_reader, &mut dist).unwrap();
            assert_eq!(dist.as_slice(), b"some text");
        }
        assert_eq!(read_options.cached_key_count(), 2);
    }

    /// Precondition: multiple encrypted solid blocks share one WriteOptions,
    /// and therefore one PHSF.
    /// Action: decode every block with one shared ReadOptions.
    /// Expectation: all blocks decode and the derived key is cached once.
    #[test]
    fn read_options_cache_is_reused_across_solid_blocks() {
        let write_options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
            .password(Some("password"))
            .try_build()
            .unwrap();
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        for i in 0..2 {
            let mut solid = SolidEntryBuilder::new(&write_options).unwrap();
            solid
                .write_file(format!("test/file-{i}").into(), Metadata::new(), |w| {
                    w.write_all(b"some text")
                })
                .unwrap();
            writer.add_entry(solid.build().unwrap()).unwrap();
        }
        let archived = writer.finalize().unwrap();

        let read_options = ReadOptions::with_password(Some("password"));
        let mut reader = Archive::read_header(archived.as_slice()).unwrap();
        for entry in reader.entries() {
            let ReadEntry::Solid(solid) = entry.unwrap() else {
                panic!("expected a solid entry");
            };
            let entries = solid
                .entries(&read_options)
                .unwrap()
                .collect::<io::Result<Vec<_>>>()
                .unwrap();
            assert_eq!(entries.len(), 1);
        }
        assert_eq!(read_options.cached_key_count(), 1);
    }

    /// Precondition: encryption-enabled WriteOptions built once.
    /// Action: create multiple archives reusing the same options.
    /// Expectation: every archive round-trips with the original password.
    #[test]
    fn write_options_reusable_across_archives() {
        let options = WriteOptions::builder()
            .encryption(Encryption::AES)
            .password(Some("password"))
            .build();
        for _ in 0..2 {
            archive(b"some text", options.clone()).unwrap();
        }
    }

    fn create_archive(src: &[u8], options: WriteOptions) -> io::Result<Vec<u8>> {
        let mut writer = Archive::write_header(Vec::with_capacity(src.len()))?;
        writer.add_entry({
            let mut builder = FileEntryBuilder::new_with_options("test/text".into(), options)?;
            builder.write_all(src)?;
            builder.build()?
        })?;
        writer.finalize()
    }

    fn archive(src: &[u8], options: WriteOptions) -> io::Result<()> {
        let read_options = ReadOptions::with_password(options.password());
        let archive = create_archive(src, options)?;
        let mut archive_reader = Archive::read_header(archive.as_slice())?;
        let item = archive_reader.entries().skip_solid().next().unwrap()?;
        let mut reader = item.reader(read_options)?;
        let mut dist = Vec::new();
        io::copy(&mut reader, &mut dist)?;
        assert_eq!(src, dist.as_slice());
        Ok(())
    }

    fn solid_archive(write_option: WriteOptions) {
        let password = write_option.password().map(|it| it.to_vec());
        let mut archive = Archive::write_solid_header(Vec::new(), write_option).unwrap();
        for i in 0..200 {
            archive
                .add_entry({
                    let mut builder =
                        FileEntryBuilder::new(format!("test/text{i}").into()).unwrap();
                    builder
                        .write_all(format!("text{i}").repeat(i).as_bytes())
                        .unwrap();
                    builder.build().unwrap()
                })
                .unwrap();
        }
        let buf = archive.finalize().unwrap();
        let mut archive = Archive::read_header(&buf[..]).unwrap();
        let mut entries = archive.entries();
        let entry = entries.next().unwrap().unwrap();
        if let ReadEntry::Solid(entry) = entry {
            let read_options = ReadOptions::with_password(password.as_deref());
            let mut entries = entry.entries(&read_options).unwrap();
            for i in 0..200 {
                let entry = entries.next().unwrap().unwrap();
                let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
                let mut body = Vec::new();
                reader.read_to_end(&mut body).unwrap();
                assert_eq!(format!("text{i}").repeat(i).as_bytes(), &body[..]);
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn solid_store_camellia_cbc() {
        solid_archive(
            WriteOptions::builder()
                .compression(Compression::NO)
                .encryption(Encryption::CAMELLIA)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("PASSWORD"))
                .build(),
        );
    }

    #[test]
    fn solid_entry() {
        let archive = {
            let mut writer = Archive::write_header(Vec::new()).unwrap();
            let dir_entry = {
                let builder = DirEntryBuilder::new("test".into());
                builder.build().unwrap()
            };
            let file_entry = {
                let options = WriteOptions::store();
                let mut builder =
                    FileEntryBuilder::new_with_options("test/text".into(), options).unwrap();
                builder.write_all(b"text").unwrap();
                builder.build().unwrap()
            };
            writer
                .add_entry({
                    let mut builder = SolidEntryBuilder::new(WriteOptions::store()).unwrap();
                    builder.add_entry(dir_entry).unwrap();
                    builder.add_entry(file_entry).unwrap();
                    builder.build().unwrap()
                })
                .unwrap();
            writer.finalize().unwrap()
        };

        let mut archive_reader = Archive::read_header(archive.as_slice()).unwrap();
        let options = ReadOptions::with_password(Some(b"password"));
        let mut entries = archive_reader.entries_with_options(&options);
        entries.next().unwrap().expect("failed to read entry");
        entries.next().unwrap().expect("failed to read entry");
        assert!(entries.next().is_none());
    }

    #[test]
    fn copy_entry() {
        let archive = create_archive(b"archive text", WriteOptions::builder().build())
            .expect("failed to create archive");
        let mut reader =
            Archive::read_header(archive.as_slice()).expect("failed to read archive header");

        let mut writer = Archive::write_header(Vec::new()).expect("failed to write archive header");

        for entry in reader.raw_entries() {
            writer
                .add_entry(entry.expect("failed to read entry"))
                .expect("failed to add entry");
        }
        assert_eq!(
            archive,
            writer.finalize().expect("failed to finish archive")
        )
    }

    #[test]
    fn append() {
        let mut writer = Archive::write_header(Vec::new()).unwrap();
        writer
            .add_entry({
                let builder = FileEntryBuilder::new_with_options(
                    "text1.txt".into(),
                    WriteOptions::builder().build(),
                )
                .unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        let result = writer.finalize().unwrap();

        let mut appender = Archive::read_header(Cursor::new(result)).unwrap();
        appender.seek_to_end().unwrap();
        appender
            .add_entry({
                let builder = FileEntryBuilder::new_with_options(
                    "text2.txt".into(),
                    WriteOptions::builder().build(),
                )
                .unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        let appended = appender.finalize().unwrap().into_inner();

        let mut reader = Archive::read_header(appended.as_slice()).unwrap();

        let mut entries = reader.entries();
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_none());
    }

    #[allow(deprecated)]
    #[test]
    fn metadata() {
        let original_entry = {
            let mut builder =
                FileEntryBuilder::new_with_options("name".into(), WriteOptions::builder().build())
                    .unwrap();
            builder.metadata(
                Metadata::new()
                    .with_created(Some(Duration::seconds(31)))
                    .with_modified(Some(Duration::seconds(32)))
                    .with_accessed(Some(Duration::seconds(33)))
                    .with_permission(Some(Permission::new(
                        1,
                        "uname".into(),
                        2,
                        "gname".into(),
                        0o775,
                    ))),
            );
            builder.write_all(b"entry data").unwrap();
            builder.build().unwrap()
        };

        let mut archive = Archive::write_header(Vec::new()).unwrap();
        archive.add_entry(original_entry.clone()).unwrap();

        let buf = archive.finalize().unwrap();

        let mut archive = Archive::read_header(buf.as_slice()).unwrap();

        let mut entries = archive.entries_with_options(&ReadOptions::builder().build());
        let read_entry = entries.next().unwrap().unwrap();

        assert_eq!(
            original_entry.metadata().created(),
            read_entry.metadata().created()
        );
        assert_eq!(
            original_entry.metadata().modified(),
            read_entry.metadata().modified()
        );
        assert_eq!(
            original_entry.metadata().accessed(),
            read_entry.metadata().accessed()
        );
        assert_eq!(
            original_entry.metadata().permission(),
            read_entry.metadata().permission()
        );
        assert_eq!(
            original_entry.metadata().compressed_size(),
            read_entry.metadata().compressed_size()
        );
        assert_eq!(
            original_entry.metadata().raw_file_size(),
            read_entry.metadata().raw_file_size()
        );
    }

    mod gcm_roundtrip {
        use super::*;
        use crate::chunk::{Chunk, ChunkExt, ChunkType, RawChunk, read_as_chunks};
        use std::num::NonZeroU32;
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        use wasm_bindgen_test::wasm_bindgen_test as test;

        fn gcm_options(
            encryption: Encryption,
            compression: Compression,
            segment_size: u32,
        ) -> WriteOptions {
            let mut builder = WriteOptions::builder();
            builder
                .compression(compression)
                .encryption(encryption)
                .cipher_mode(CipherMode::GCM)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"));
            builder.segment_size(segment_size);
            builder.build()
        }

        fn per_entry_archive(
            options: &WriteOptions,
            plain: &[u8],
            max_chunk: Option<u32>,
        ) -> Vec<u8> {
            let mut writer = Archive::write_header(Vec::new()).unwrap();
            let mut builder =
                FileEntryBuilder::new_with_options("dir/file".into(), options).unwrap();
            if let Some(size) = max_chunk {
                builder.max_chunk_size(NonZeroU32::new(size).unwrap());
            }
            builder.write_all(plain).unwrap();
            writer.add_entry(builder.build().unwrap()).unwrap();
            writer.finalize().unwrap()
        }

        fn read_per_entry(archived: &[u8], password: Option<&str>) -> io::Result<Vec<u8>> {
            let mut reader = Archive::read_header(archived).unwrap();
            let entry = reader.entries().skip_solid().next().unwrap().unwrap();
            let mut r = entry.reader(ReadOptions::with_password(password))?;
            let mut out = Vec::new();
            r.read_to_end(&mut out)?;
            Ok(out)
        }

        /// Rewrites a single-entry archive so that its datastream is carried by
        /// `FDAT` chunks split at the given byte offsets, regardless of how the
        /// encoder originally chunked it.
        fn resplit_datastream(archive: &[u8], boundaries: &[usize]) -> Vec<u8> {
            let mut stream = Vec::new();
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                if chunk.ty() == ChunkType::FDAT {
                    stream.extend_from_slice(chunk.data());
                }
            }
            let mut segments = Vec::new();
            let mut prev = 0;
            for &b in boundaries {
                segments.push(&stream[prev..b]);
                prev = b;
            }
            segments.push(&stream[prev..]);

            let mut out = PNA_HEADER.to_vec();
            let mut emitted = false;
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                if chunk.ty() == ChunkType::FDAT {
                    if !emitted {
                        for segment in &segments {
                            RawChunk::from_data(ChunkType::FDAT, segment.to_vec())
                                .write_chunk_in(&mut out)
                                .unwrap();
                        }
                        emitted = true;
                    }
                    continue;
                }
                RawChunk::from_data(chunk.ty(), chunk.data().to_vec())
                    .write_chunk_in(&mut out)
                    .unwrap();
            }
            out
        }

        fn assert_per_entry_roundtrip(
            encryption: Encryption,
            compression: Compression,
            plain: &[u8],
        ) {
            let options = gcm_options(encryption, compression, 4);
            let archived = per_entry_archive(&options, plain, None);
            assert_eq!(read_per_entry(&archived, Some("password")).unwrap(), plain);
        }

        fn assert_solid_roundtrip(encryption: Encryption, compression: Compression, plain: &[u8]) {
            let options = gcm_options(encryption, compression, 4);
            let mut writer = Archive::write_solid_header(Vec::new(), options).unwrap();
            writer
                .add_entry({
                    let mut b = FileEntryBuilder::new("dir/file".into()).unwrap();
                    b.write_all(plain).unwrap();
                    b.build().unwrap()
                })
                .unwrap();
            let archived = writer.finalize().unwrap();

            let mut reader = Archive::read_header(archived.as_slice()).unwrap();
            let entry = reader.entries().next().unwrap().unwrap();
            let ReadEntry::Solid(solid) = entry else {
                panic!("expected a solid entry");
            };
            let options = ReadOptions::with_password(Some(b"password"));
            let inner = solid.entries(&options).unwrap().next().unwrap().unwrap();
            let mut r = inner.reader(ReadOptions::builder().build()).unwrap();
            let mut out = Vec::new();
            r.read_to_end(&mut out).unwrap();
            assert_eq!(out, plain);
        }

        const REPRESENTATIVE: &[u8] = b"012345678";

        #[test]
        fn per_entry_aes_uncompressed() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::NO, REPRESENTATIVE);
        }

        #[test]
        fn per_entry_aes_zstd() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::ZSTANDARD, REPRESENTATIVE);
        }

        #[test]
        fn per_entry_camellia_uncompressed() {
            assert_per_entry_roundtrip(Encryption::CAMELLIA, Compression::NO, REPRESENTATIVE);
        }

        #[test]
        fn per_entry_camellia_zstd() {
            assert_per_entry_roundtrip(
                Encryption::CAMELLIA,
                Compression::ZSTANDARD,
                REPRESENTATIVE,
            );
        }

        #[test]
        fn solid_aes_uncompressed() {
            assert_solid_roundtrip(Encryption::AES, Compression::NO, REPRESENTATIVE);
        }

        #[test]
        fn solid_aes_zstd() {
            assert_solid_roundtrip(Encryption::AES, Compression::ZSTANDARD, REPRESENTATIVE);
        }

        #[test]
        fn solid_camellia_uncompressed() {
            assert_solid_roundtrip(Encryption::CAMELLIA, Compression::NO, REPRESENTATIVE);
        }

        #[test]
        fn solid_camellia_zstd() {
            assert_solid_roundtrip(Encryption::CAMELLIA, Compression::ZSTANDARD, REPRESENTATIVE);
        }

        #[test]
        fn plaintext_length_zero() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::NO, b"");
        }

        #[test]
        fn plaintext_length_below_segment() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::NO, b"abc");
        }

        #[test]
        fn plaintext_length_exact_segment() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::NO, b"abcd");
        }

        #[test]
        fn plaintext_length_two_segments() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::NO, b"abcdefgh");
        }

        #[test]
        fn plaintext_length_partial_tail() {
            assert_per_entry_roundtrip(Encryption::AES, Compression::NO, b"abcdefghi");
        }

        #[test]
        fn chunk_split_default() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let archived = per_entry_archive(&options, REPRESENTATIVE, None);
            assert_eq!(
                read_per_entry(&archived, Some("password")).unwrap(),
                REPRESENTATIVE
            );
        }

        #[test]
        fn chunk_split_one_byte() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let archived = per_entry_archive(&options, REPRESENTATIVE, Some(1));
            assert_eq!(
                read_per_entry(&archived, Some("password")).unwrap(),
                REPRESENTATIVE
            );
        }

        #[test]
        fn fdat_boundary_inside_stream_header() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let archived = per_entry_archive(&options, REPRESENTATIVE, None);
            let resplit = resplit_datastream(&archived, &[20]);
            assert_eq!(
                read_per_entry(&resplit, Some("password")).unwrap(),
                REPRESENTATIVE
            );
        }

        #[test]
        fn fdat_boundary_inside_segment_tag() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let archived = per_entry_archive(&options, REPRESENTATIVE, None);
            // 43-byte header + 4-byte first-segment ciphertext + 8 bytes into its
            // 16-byte GCM tag.
            let resplit = resplit_datastream(&archived, &[43 + 4 + 8]);
            assert_eq!(
                read_per_entry(&resplit, Some("password")).unwrap(),
                REPRESENTATIVE
            );
        }

        #[test]
        fn slice_path_reads_gcm_entry() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let archived = per_entry_archive(&options, REPRESENTATIVE, None);
            let mut reader = Archive::read_header_from_slice(&archived).unwrap();
            let entry = reader.entries_slice().next().unwrap().unwrap();
            let ReadEntry::Normal(entry) = entry else {
                panic!("expected a normal entry");
            };
            let mut r = entry
                .reader(ReadOptions::with_password(Some("password")))
                .unwrap();
            let mut out = Vec::new();
            r.read_to_end(&mut out).unwrap();
            assert_eq!(out, REPRESENTATIVE);
        }

        #[test]
        fn entries_with_options_reads_solid_gcm_entry() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let mut writer = Archive::write_solid_header(Vec::new(), options).unwrap();
            writer
                .add_entry({
                    let mut b = FileEntryBuilder::new("dir/file".into()).unwrap();
                    b.write_all(REPRESENTATIVE).unwrap();
                    b.build().unwrap()
                })
                .unwrap();
            let archived = writer.finalize().unwrap();

            let mut reader = Archive::read_header(archived.as_slice()).unwrap();
            let entry = reader
                .entries_with_options(&ReadOptions::with_password(Some(b"password")))
                .next()
                .unwrap()
                .unwrap();
            let mut r = entry.reader(ReadOptions::builder().build()).unwrap();
            let mut out = Vec::new();
            r.read_to_end(&mut out).unwrap();
            assert_eq!(out, REPRESENTATIVE);
        }

        /// Decryption binds to the raw header bytes while re-serialization uses
        /// the parsed header, so copying an entry into a new archive must keep
        /// the two representations identical.
        #[test]
        fn copied_entry_remains_decryptable() {
            let options = gcm_options(Encryption::AES, Compression::NO, 4);
            let archived = per_entry_archive(&options, REPRESENTATIVE, None);

            let mut reader = Archive::read_header(archived.as_slice()).unwrap();
            let mut writer = Archive::write_header(Vec::new()).unwrap();
            for entry in reader.entries().skip_solid() {
                writer.add_entry(entry.unwrap()).unwrap();
            }
            let copied = writer.finalize().unwrap();

            assert_eq!(
                read_per_entry(&copied, Some("password")).unwrap(),
                REPRESENTATIVE
            );
        }
    }

    mod gcm_negative {
        use super::*;
        use crate::chunk::{Chunk, ChunkExt, ChunkType, RawChunk, read_as_chunks};
        use crate::error::AeadError;
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        use wasm_bindgen_test::wasm_bindgen_test as test;

        const PASSWORD: &str = "password";
        const SEGMENT_SIZE: u32 = 4;
        const STREAM_HEADER_LEN: usize = 43;
        const GCM_TAG_LEN: usize = 16;

        fn gcm_options() -> WriteOptions {
            gcm_options_with_segment(SEGMENT_SIZE)
        }

        fn gcm_options_with_segment(segment_size: u32) -> WriteOptions {
            let mut builder = WriteOptions::builder();
            builder
                .compression(Compression::NO)
                .encryption(Encryption::AES)
                .cipher_mode(CipherMode::GCM)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some(PASSWORD));
            builder.segment_size(segment_size);
            builder.build()
        }

        fn archive_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
            let options = gcm_options();
            let mut writer = Archive::write_header(Vec::new()).unwrap();
            for (path, plain) in entries {
                let mut builder =
                    FileEntryBuilder::new_with_options((*path).into(), &options).unwrap();
                builder.write_all(plain).unwrap();
                writer.add_entry(builder.build().unwrap()).unwrap();
            }
            writer.finalize().unwrap()
        }

        fn single(plain: &[u8]) -> Vec<u8> {
            archive_of(&[("dir/file", plain)])
        }

        fn read_nth(archive: &[u8], index: usize, password: Option<&str>) -> io::Result<Vec<u8>> {
            let mut reader = Archive::read_header(archive).unwrap();
            let entry = reader.entries().skip_solid().nth(index).unwrap().unwrap();
            let mut r = entry.reader(ReadOptions::with_password(password))?;
            let mut out = Vec::new();
            r.read_to_end(&mut out)?;
            Ok(out)
        }

        fn read_first(archive: &[u8]) -> io::Result<Vec<u8>> {
            read_nth(archive, 0, Some(PASSWORD))
        }

        fn tamper_chunk(
            archive: &[u8],
            chunk_type: ChunkType,
            index: usize,
            f: impl FnOnce(&mut Vec<u8>),
        ) -> Vec<u8> {
            let mut out = PNA_HEADER.to_vec();
            let mut f = Some(f);
            let mut seen = 0;
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                let mut data = chunk.data().to_vec();
                if chunk.ty() == chunk_type {
                    if seen == index {
                        (f.take().expect("target chunk is tampered once"))(&mut data);
                    }
                    seen += 1;
                }
                RawChunk::from_data(chunk.ty(), data)
                    .write_chunk_in(&mut out)
                    .unwrap();
            }
            assert!(f.is_none(), "target chunk was found and tampered");
            out
        }

        fn datastream_of(archive: &[u8], entry_index: usize) -> Vec<u8> {
            let mut current: isize = -1;
            let mut stream = Vec::new();
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                if chunk.ty() == ChunkType::FHED {
                    current += 1;
                }
                if chunk.ty() == ChunkType::FDAT && current == entry_index as isize {
                    stream.extend_from_slice(chunk.data());
                }
            }
            stream
        }

        fn tamper_datastream(
            archive: &[u8],
            entry_index: usize,
            f: impl FnOnce(&mut Vec<u8>),
        ) -> Vec<u8> {
            let mut out = PNA_HEADER.to_vec();
            let mut f = Some(f);
            let mut current: isize = -1;
            let mut stream: Option<Vec<u8>> = None;
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                let ty = chunk.ty();
                if ty == ChunkType::FHED {
                    current += 1;
                }
                if ty == ChunkType::FDAT && current == entry_index as isize {
                    stream
                        .get_or_insert_with(Vec::new)
                        .extend_from_slice(chunk.data());
                    continue;
                }
                if let Some(mut data) = stream.take() {
                    (f.take().expect("target datastream is tampered once"))(&mut data);
                    RawChunk::from_data(ChunkType::FDAT, data)
                        .write_chunk_in(&mut out)
                        .unwrap();
                }
                RawChunk::from_data(ty, chunk.data().to_vec())
                    .write_chunk_in(&mut out)
                    .unwrap();
            }
            assert!(f.is_none(), "target datastream was found and tampered");
            out
        }

        fn aead(err: &io::Error) -> &AeadError {
            err.get_ref()
                .and_then(|e| e.downcast_ref::<AeadError>())
                .expect("decrypt error carries an AeadError")
        }

        fn solid_datastream(archive: &[u8]) -> Vec<u8> {
            let mut stream = Vec::new();
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                if chunk.ty() == ChunkType::SDAT {
                    stream.extend_from_slice(chunk.data());
                }
            }
            stream
        }

        fn tamper_solid_datastream(archive: &[u8], f: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
            let mut out = PNA_HEADER.to_vec();
            let mut f = Some(f);
            let mut stream: Option<Vec<u8>> = None;
            for chunk in read_as_chunks(archive).unwrap() {
                let chunk = chunk.unwrap();
                let ty = chunk.ty();
                if ty == ChunkType::SDAT {
                    stream
                        .get_or_insert_with(Vec::new)
                        .extend_from_slice(chunk.data());
                    continue;
                }
                if let Some(mut data) = stream.take() {
                    (f.take().expect("target datastream is tampered once"))(&mut data);
                    RawChunk::from_data(ChunkType::SDAT, data)
                        .write_chunk_in(&mut out)
                        .unwrap();
                }
                RawChunk::from_data(ty, chunk.data().to_vec())
                    .write_chunk_in(&mut out)
                    .unwrap();
            }
            assert!(f.is_none(), "target datastream was found and tampered");
            out
        }

        /// Reads inner entry contents, stopping at the first error (the iterator
        /// reproduces a decryption error indefinitely rather than ending).
        fn read_solid_contents(archive: &[u8], password: Option<&str>) -> Vec<io::Result<Vec<u8>>> {
            let mut reader = Archive::read_header(archive).unwrap();
            let ReadEntry::Solid(solid) = reader.entries().next().unwrap().unwrap() else {
                panic!("expected a solid entry");
            };
            let mut contents = Vec::new();
            let options = ReadOptions::with_password(password);
            for entry in solid.entries(&options).unwrap() {
                let result = entry.and_then(|e| {
                    let mut r = e.reader(ReadOptions::builder().build())?;
                    let mut out = Vec::new();
                    r.read_to_end(&mut out)?;
                    Ok(out)
                });
                let stop = result.is_err();
                contents.push(result);
                if stop {
                    break;
                }
            }
            contents
        }

        #[test]
        fn wrong_password_fails_authentication_without_distinguishing_the_cause() {
            let archive = single(b"abcdefgh");
            assert_eq!(read_first(&archive).unwrap(), b"abcdefgh");
            let err = read_nth(&archive, 0, Some("wrong")).unwrap_err();
            assert!(matches!(aead(&err), AeadError::AuthenticationFailure));
            let message = err.to_string();
            assert!(message.contains("wrong password"));
            assert!(message.contains("corrupted data"));
        }

        /// K_master is shared across entries written with the same options, so
        /// per-stream key and nonce uniqueness rests entirely on the random
        /// stream salt and nonce prefix in each stream header.
        #[test]
        fn entries_use_distinct_stream_salts_and_nonce_prefixes() {
            let archive = archive_of(&[("dir/a", b"abcdefgh"), ("dir/b", b"abcdefgh")]);
            let first = datastream_of(&archive, 0);
            let second = datastream_of(&archive, 1);
            assert_ne!(first[..32], second[..32], "stream salts must differ");
            assert_ne!(first[32..39], second[32..39], "nonce prefixes must differ");
        }

        #[test]
        fn truncating_solid_datastream_at_inner_entry_boundary_is_truncation() {
            let entry = |path: &str, plain: &[u8]| {
                let mut b = FileEntryBuilder::new(path.into()).unwrap();
                b.write_all(plain).unwrap();
                b.build().unwrap()
            };
            let first = entry("dir/a", b"first");
            let second = entry("dir/b", b"second");

            let mut writer =
                Archive::write_solid_header(Vec::new(), WriteOptions::store()).unwrap();
            writer.add_entry(first.clone()).unwrap();
            let first_len = solid_datastream(&writer.finalize().unwrap()).len();

            // The first GCM segment ends exactly at the first inner entry's FEND.
            let mut writer =
                Archive::write_solid_header(Vec::new(), gcm_options_with_segment(first_len as u32))
                    .unwrap();
            writer.add_entry(first).unwrap();
            writer.add_entry(second).unwrap();
            let archived = writer.finalize().unwrap();
            let contents = read_solid_contents(&archived, Some(PASSWORD));
            assert_eq!(
                contents
                    .into_iter()
                    .map(|it| it.unwrap())
                    .collect::<Vec<_>>(),
                [b"first".to_vec(), b"second".to_vec()]
            );

            let truncated = tamper_solid_datastream(&archived, |data| {
                data.truncate(STREAM_HEADER_LEN + first_len + GCM_TAG_LEN + 8);
            });
            let results = read_solid_contents(&truncated, Some(PASSWORD));
            assert_eq!(results[0].as_ref().unwrap(), b"first");
            assert_eq!(
                results.len(),
                2,
                "truncation must surface as an error instead of ending iteration cleanly"
            );
            assert!(matches!(
                aead(results[1].as_ref().unwrap_err()),
                AeadError::Truncation
            ));
        }

        #[test]
        fn swapping_datastreams_between_entries_fails_authentication() {
            let archive = archive_of(&[("dir/a", b"abcdefgh"), ("dir/b", b"abcdefgh")]);
            assert_eq!(read_nth(&archive, 0, Some(PASSWORD)).unwrap(), b"abcdefgh");
            assert_eq!(read_nth(&archive, 1, Some(PASSWORD)).unwrap(), b"abcdefgh");
            let stream0 = datastream_of(&archive, 0);
            let stream1 = datastream_of(&archive, 1);
            let archive = tamper_datastream(&archive, 0, |data| *data = stream1);
            let archive = tamper_datastream(&archive, 1, |data| *data = stream0);
            let err = read_nth(&archive, 0, Some(PASSWORD)).unwrap_err();
            assert!(matches!(aead(&err), AeadError::AuthenticationFailure));
        }

        #[test]
        fn modifying_phsf_salt_fails_authentication() {
            let archive = single(b"abcdefgh");
            assert_eq!(read_first(&archive).unwrap(), b"abcdefgh");
            let tampered = tamper_chunk(&archive, ChunkType::PHSF, 0, |data| {
                let phsf = std::str::from_utf8(data).unwrap();
                let mut fields = phsf.split('$').map(str::to_owned).collect::<Vec<_>>();
                let salt = &mut fields[3];
                let mut bytes = std::mem::take(salt).into_bytes();
                bytes[0] = if bytes[0] == b'A' { b'B' } else { b'A' };
                *salt = String::from_utf8(bytes).unwrap();
                *data = fields.join("$").into_bytes();
            });
            let err = read_first(&tampered).unwrap_err();
            assert!(matches!(aead(&err), AeadError::AuthenticationFailure));
        }

        #[test]
        fn modifying_fhed_path_fails_authentication() {
            let archive = single(b"abcdefgh");
            assert_eq!(read_first(&archive).unwrap(), b"abcdefgh");
            let tampered = tamper_chunk(&archive, ChunkType::FHED, 0, |data| {
                data[6] ^= 0x01;
            });
            let err = read_first(&tampered).unwrap_err();
            assert!(matches!(aead(&err), AeadError::AuthenticationFailure));
        }

        #[test]
        fn datastream_shorter_than_the_stream_header_is_malformed() {
            let archive = single(b"012345678");
            assert_eq!(read_first(&archive).unwrap(), b"012345678");
            let tampered = tamper_datastream(&archive, 0, |data| {
                data.truncate(STREAM_HEADER_LEN - 1);
            });
            let err = read_first(&tampered).unwrap_err();
            assert!(matches!(aead(&err), AeadError::Malformed(_)));
        }
    }
}
