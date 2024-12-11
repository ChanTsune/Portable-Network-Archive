mod header;
mod read;
mod write;

use crate::{
    chunk::{ChunkStreamWriter, RawChunk},
    cipher::CipherWriter,
    compress::CompressionWriter,
};
pub use header::*;
pub(crate) use read::*;
use std::io::prelude::*;

/// An object providing access to a PNA file.
/// An instance of an [Archive] can be read and/or written.
///
/// # Examples
/// Creates a new PNA file and adds entry to it.
/// ```no_run
/// # use libpna::{Archive, EntryBuilder, WriteOptions};
/// # use std::fs::File;
/// # use std::io::{self, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let file = File::create("foo.pna")?;
/// let mut archive = Archive::write_header(file)?;
/// let mut entry_builder =
///     EntryBuilder::new_file("bar.txt".into(), WriteOptions::builder().build())?;
/// entry_builder.write_all(b"content")?;
/// let entry = entry_builder.build()?;
/// archive.add_entry(entry)?;
/// archive.finalize()?;
/// #     Ok(())
/// # }
/// ```
///
/// Read the entries of a pna file.
/// ```no_run
/// # use libpna::{Archive, ReadOptions};
/// # use std::fs::File;
/// # use std::io::{self, copy, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let file = File::open("foo.pna")?;
/// let mut archive = Archive::read_header(file)?;
/// for entry in archive.entries_skip_solid() {
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
            next_archive: false,
            buf,
        }
    }

    /// Returns `true` if [ANXT] chunk is appeared before call this method calling.
    ///
    /// # Returns
    ///
    /// `true` if the next archive in the series is available, otherwise `false`.
    ///
    /// [ANXT]: crate::chunk::ChunkType::ANXT
    #[inline]
    pub const fn has_next_archive(&self) -> bool {
        self.next_archive
    }
}

/// An object providing writing to a solid mode PNA file.
/// An instance of an [SolidArchive] can be written.
///
/// # Examples
/// Creates a new solid mode PNA file and adds entry to it.
/// ```no_run
/// use libpna::{Archive, EntryBuilder, WriteOptions};
/// use std::fs::File;
/// # use std::io::{self, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let option = WriteOptions::builder().build();
/// let file = File::create("foo.pna")?;
/// let mut archive = Archive::write_solid_header(file, option)?;
/// let mut entry_builder = EntryBuilder::new_file("bar.txt".into(), WriteOptions::store())?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::*;
    use std::io::{self, Cursor};
    use std::time::Duration;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn store_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder().compression(Compression::No).build(),
        )
        .unwrap()
    }

    #[test]
    fn deflate_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder()
                .compression(Compression::Deflate)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn zstd_archive() {
        archive(
            b"src data bytes",
            WriteOptions::builder()
                .compression(Compression::ZStandard)
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
                .compression(Compression::No)
                .encryption(Encryption::Aes)
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
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
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
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
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
                .compression(Compression::ZStandard)
                .encryption(Encryption::Camellia)
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
                .compression(Compression::ZStandard)
                .encryption(Encryption::Camellia)
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
                .encryption(Encryption::Aes)
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
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::pbkdf2_sha256_with(Some(1)))
                .password(Some("password"))
                .build(),
        )
        .unwrap()
    }

    fn create_archive(src: &[u8], options: WriteOptions) -> io::Result<Vec<u8>> {
        let mut writer = Archive::write_header(Vec::with_capacity(src.len()))?;
        writer.add_entry({
            let mut builder = EntryBuilder::new_file("test/text".into(), options)?;
            builder.write_all(src)?;
            builder.build()?
        })?;
        writer.finalize()
    }

    fn archive(src: &[u8], options: WriteOptions) -> io::Result<()> {
        let read_options = ReadOptions::with_password(options.password());
        let archive = create_archive(src, options)?;
        let mut archive_reader = Archive::read_header(archive.as_slice())?;
        let item = archive_reader.entries_skip_solid().next().unwrap().unwrap();
        let mut reader = item.reader(read_options).unwrap();
        let mut dist = Vec::new();
        io::copy(&mut reader, &mut dist)?;
        assert_eq!(src, dist.as_slice());
        Ok(())
    }

    fn solid_archive(write_option: WriteOptions) {
        let password = write_option.password().map(|it| it.to_string());
        let mut archive = Archive::write_solid_header(Vec::new(), write_option).unwrap();
        for i in 0..200 {
            archive
                .add_entry({
                    let mut builder = EntryBuilder::new_file(
                        format!("test/text{i}").as_str().into(),
                        WriteOptions::store(),
                    )
                    .unwrap();
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
            let mut entries = entry.entries(password.as_deref()).unwrap();
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
                .compression(Compression::No)
                .encryption(Encryption::Camellia)
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
                let builder = EntryBuilder::new_dir("test".into());
                builder.build().unwrap()
            };
            let file_entry = {
                let options = WriteOptions::store();
                let mut builder = EntryBuilder::new_file("test/text".into(), options).unwrap();
                builder.write_all("text".as_bytes()).unwrap();
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
        let mut entries = archive_reader.entries_with_password(Some("password"));
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
                let builder =
                    EntryBuilder::new_file("text1.txt".into(), WriteOptions::builder().build())
                        .unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        let result = writer.finalize().unwrap();

        let mut appender = Archive::read_header(Cursor::new(result)).unwrap();
        appender.seek_to_end().unwrap();
        appender
            .add_entry({
                let builder =
                    EntryBuilder::new_file("text2.txt".into(), WriteOptions::builder().build())
                        .unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        let appended = appender.finalize().unwrap().into_inner();

        let mut reader = Archive::read_header(appended.as_slice()).unwrap();

        let mut entries = reader.entries_skip_solid();
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_none());
    }

    #[test]
    fn metadata() {
        let original_entry = {
            let mut builder =
                EntryBuilder::new_file("name".into(), WriteOptions::builder().build()).unwrap();
            builder.created(Duration::from_secs(31));
            builder.modified(Duration::from_secs(32));
            builder.accessed(Duration::from_secs(33));
            builder.permission(Permission::new(1, "uname".into(), 2, "gname".into(), 0o775));
            builder.write_all(b"entry data").unwrap();
            builder.build().unwrap()
        };

        let mut archive = Archive::write_header(Vec::new()).unwrap();
        archive.add_entry(original_entry.clone()).unwrap();

        let buf = archive.finalize().unwrap();

        let mut archive = Archive::read_header(buf.as_slice()).unwrap();

        let mut entries = archive.entries_with_password(None);
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
}
