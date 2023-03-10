mod entry;
mod header;
mod read;
mod write;

pub use entry::{
    CipherMode, Compression, CompressionLevel, DataKind, Encryption, Entry, EntryHeader,
    HashAlgorithm, ItemName, ReadOption, ReadOptionBuilder, WriteEntry, WriteOption,
    WriteOptionBuilder,
};
pub use header::PNA_HEADER;
pub use read::{ArchiveReader, Decoder};
pub use write::{ArchiveWriter, Encoder};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

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

    fn archive(src: &[u8], options: WriteOption) -> io::Result<()> {
        let mut archived_temp = Vec::new();
        {
            let encoder = Encoder::new();
            let mut archive_writer = encoder.write_header(&mut archived_temp)?;
            archive_writer.start_file_with_options("test/text".into(), options.clone())?;
            archive_writer.write_all(src)?;
            archive_writer.end_file()?;
            archive_writer.finalize()?;
        }
        let mut dist = Vec::new();
        let decoder = Decoder::new();
        let mut archive_reader = decoder.read_header(io::Cursor::new(archived_temp))?;
        let mut item = archive_reader
            .read()
            .unwrap()
            .unwrap()
            .to_reader({
                let mut builder = ReadOptionBuilder::new();
                if let Some(password) = options.password {
                    builder.password(password);
                }
                builder.build()
            })
            .unwrap();
        io::copy(&mut item, &mut dist)?;
        assert_eq!(src, dist.as_slice());
        Ok(())
    }
}
