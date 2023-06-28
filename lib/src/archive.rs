mod entry;
mod header;
mod read;
mod write;

pub use entry::*;
pub use header::*;
pub use read::*;
pub use write::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    #[test]
    fn store_archive() {
        archive(
            b"src data bytes",
            WriteOptionBuilder::default()
                .compression(Compression::No)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn deflate_archive() {
        archive(
            b"src data bytes",
            WriteOptionBuilder::default()
                .compression(Compression::Deflate)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn zstd_archive() {
        archive(
            b"src data bytes",
            WriteOptionBuilder::default()
                .compression(Compression::ZStandard)
                .build(),
        )
        .unwrap()
    }

    #[test]
    fn xz_archive() {
        archive(
            b"src data bytes",
            WriteOptionBuilder::default()
                .compression(Compression::XZ)
                .build(),
        )
        .unwrap();
    }

    #[test]
    fn store_with_aes_cbc_archive() {
        archive(
            b"plain text",
            WriteOptionBuilder::default()
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
            WriteOptionBuilder::default()
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
            WriteOptionBuilder::default()
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
            WriteOptionBuilder::default()
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
            WriteOptionBuilder::default()
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
            WriteOptionBuilder::default()
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
            WriteOptionBuilder::default()
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
        let mut writer = ArchiveWriter::write_header(Vec::with_capacity(src.len()))?;
        writer.add_entry({
            let mut builder = EntryBuilder::new_file("test/text".try_into().unwrap(), options)?;
            builder.write_all(src)?;
            builder.build()?
        })?;
        writer.finalize()
    }

    fn archive(src: &[u8], options: WriteOption) -> io::Result<()> {
        let archive = create_archive(src, options.clone())?;
        let mut archive_reader = ArchiveReader::read_header(io::Cursor::new(archive))?;
        let mut item = archive_reader
            .entries()
            .next()
            .unwrap()
            .unwrap()
            .into_reader({
                let mut builder = ReadOption::builder();
                if let Some(password) = options.password {
                    builder.password(password);
                }
                builder.build()
            })
            .unwrap();
        let mut dist = Vec::new();
        io::copy(&mut item, &mut dist)?;
        assert_eq!(src, dist.as_slice());
        Ok(())
    }

    #[test]
    fn solid_entry() -> Result<(), Box<dyn std::error::Error>> {
        let archive = {
            let mut writer = ArchiveWriter::write_header(Vec::new())?;
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
            writer.add_solid_entries({
                let mut builder = SolidEntriesBuilder::new(WriteOption::builder().build()).unwrap();
                builder.add_entry(dir_entry).unwrap();
                builder.add_entry(file_entry).unwrap();
                builder.build().unwrap()
            })?;
            writer.finalize().unwrap()
        };

        let mut archive_reader = ArchiveReader::read_header(io::Cursor::new(archive)).unwrap();
        let mut entries = archive_reader.entries_with_password(Some("password".to_string()));
        entries.next().unwrap().expect("failed to read entry");
        entries.next().unwrap().expect("failed to read entry");
        assert!(entries.next().is_none());
        Ok(())
    }

    #[test]
    fn copy_entry() {
        let archive = create_archive(b"archive text", WriteOption::builder().build())
            .expect("failed to create archive");
        let mut reader = ArchiveReader::read_header(io::Cursor::new(&archive))
            .expect("failed to read archive header");

        let mut writer =
            ArchiveWriter::write_header(Vec::new()).expect("failed to write archive header");

        for entry in reader.entries() {
            writer
                .add_entry(entry.expect("failed to read entry"))
                .expect("failed to add entry");
        }
        assert_eq!(
            archive,
            writer.finalize().expect("failed to finish archive")
        )
    }
}
