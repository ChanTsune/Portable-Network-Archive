mod entry;
mod header;
mod read;
mod write;

use crate::chunk::RawChunk;
pub use entry::*;
pub use header::*;

/// An object providing access to a PNA file.
/// An instance of an [Archive] can be read and/or written.
///
/// # Examples
/// Creates a new PNA file and add entry to it.
/// ```no_run
/// # use libpna::{Archive, EntryBuilder, WriteOption};
/// # use std::fs::File;
/// # use std::io::{self, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let file = File::create("foo.pna")?;
/// let mut archive = Archive::write_header(file)?;
/// let mut entry_builder = EntryBuilder::new_file(
///     "bar.txt".try_into().unwrap(),
///     WriteOption::builder().build(),
/// )?;
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
/// # use libpna::{Archive, ReadOption};
/// # use std::fs::File;
/// # use std::io::{self, copy, prelude::*};
///
/// # fn main() -> io::Result<()> {
/// let file = File::open("foo.pna")?;
/// let mut archive = Archive::read_header(file)?;
/// for entry in archive.entries_skip_solid() {
///     let entry = entry?;
///     let mut file = File::create(entry.header().path().as_path())?;
///     let mut reader = entry.reader(ReadOption::builder().build())?;
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
    fn new(inner: T, header: ArchiveHeader) -> Self {
        Self::with_buffer(inner, header, Default::default())
    }

    fn with_buffer(inner: T, header: ArchiveHeader, buf: Vec<RawChunk>) -> Self {
        Self {
            inner,
            header,
            next_archive: false,
            buf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor, Write};
    use std::time::Duration;

    #[test]
    fn store_archive() {
        archive(
            b"src data bytes",
            WriteOption::builder().compression(Compression::No).build(),
        )
        .unwrap()
    }

    #[test]
    fn deflate_archive() {
        archive(
            b"src data bytes",
            WriteOption::builder()
                .compression(Compression::Deflate)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn zstd_archive() {
        archive(
            b"src data bytes",
            WriteOption::builder()
                .compression(Compression::ZStandard)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn xz_archive() {
        archive(
            b"src data bytes",
            WriteOption::builder().compression(Compression::XZ).build(),
        )
        .unwrap();
    }

    #[test]
    fn store_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::No)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC)
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_aes_ctr_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CTR)
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC)
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_camellia_ctr_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CTR)
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_camellia_cbc_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC)
                .password(Some("password"))
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn xz_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::XZ)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::Pbkdf2Sha256)
                .password(Some("password"))
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn xz_with_camellia_cbc_archive() {
        archive(
            b"plain text",
            WriteOption::builder()
                .compression(Compression::XZ)
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::Pbkdf2Sha256)
                .password(Some("password"))
                .build(),
        )
        .unwrap()
    }

    fn create_archive(src: &[u8], options: WriteOption) -> io::Result<Vec<u8>> {
        let mut writer = Archive::write_header(Vec::with_capacity(src.len()))?;
        writer.add_entry({
            let mut builder = EntryBuilder::new_file("test/text".try_into().unwrap(), options)?;
            builder.write_all(src)?;
            builder.build()?
        })?;
        writer.finalize()
    }

    fn archive(src: &[u8], options: WriteOption) -> io::Result<()> {
        let archive = create_archive(src, options.clone())?;
        let mut archive_reader = Archive::read_header(archive.as_slice())?;
        let item = archive_reader.entries_skip_solid().next().unwrap().unwrap();
        let mut reader = item
            .reader(ReadOption::with_password(options.password))
            .unwrap();
        let mut dist = Vec::new();
        io::copy(&mut reader, &mut dist)?;
        assert_eq!(src, dist.as_slice());
        Ok(())
    }

    #[test]
    fn solid_entry() -> Result<(), Box<dyn std::error::Error>> {
        let archive = {
            let mut writer = Archive::write_header(Vec::new())?;
            let dir_entry = {
                let builder = EntryBuilder::new_dir("test".try_into().unwrap());
                builder.build().unwrap()
            };
            let file_entry = {
                let options = WriteOption::builder().build();
                let mut builder = EntryBuilder::new_file("test/text".try_into().unwrap(), options)?;
                builder.write_all("text".as_bytes())?;
                builder.build()?
            };
            writer.add_entry({
                let mut builder = SolidEntryBuilder::new(WriteOption::builder().build()).unwrap();
                builder.add_entry(dir_entry).unwrap();
                builder.add_entry(file_entry).unwrap();
                builder.build().unwrap()
            })?;
            writer.finalize().unwrap()
        };

        let mut archive_reader = Archive::read_header(archive.as_slice()).unwrap();
        let mut entries = archive_reader.entries_with_password(Some("password"));
        entries.next().unwrap().expect("failed to read entry");
        entries.next().unwrap().expect("failed to read entry");
        assert!(entries.next().is_none());
        Ok(())
    }

    #[test]
    fn copy_entry() {
        let archive = create_archive(b"archive text", WriteOption::builder().build())
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
                let builder = EntryBuilder::new_file(
                    EntryName::from_lossy("text1.txt"),
                    WriteOption::builder().build(),
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
                let builder = EntryBuilder::new_file(
                    EntryName::from_lossy("text2.txt"),
                    WriteOption::builder().build(),
                )
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
            let mut builder = EntryBuilder::new_file(
                EntryName::from_lossy("name"),
                WriteOption::builder().build(),
            )
            .unwrap();
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
