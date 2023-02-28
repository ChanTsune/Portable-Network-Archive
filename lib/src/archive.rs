mod header;
mod item;
mod read;
mod write;

pub use header::PNA_HEADER;
pub use item::{
    CipherMode, Compression, CompressionLevel, DataKind, Encryption, HashAlgorithm, Item, ItemInfo,
    Options,
};
pub use read::{ArchiveReader, Decoder};
pub use write::{ArchiveWriter, Encoder};

#[cfg(test)]
mod tests {
    use super::{CipherMode, Compression, Decoder, Encoder, Encryption, HashAlgorithm, Options};
    use std::io;

    #[test]
    fn store_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::No),
        )
        .unwrap()
    }

    #[test]
    fn deflate_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::Deflate),
        )
        .unwrap()
    }

    #[test]
    fn zstd_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::ZStandard),
        )
        .unwrap()
    }

    #[test]
    fn xz_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::XZ),
        )
        .unwrap();
    }

    #[test]
    fn store_with_aes_cbc_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::No)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC)
                .password(Some("password")),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_aes_ctr_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CTR)
                .password(Some("password")),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_aes_cbc_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC)
                .password(Some("password")),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_camellia_ctr_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CTR)
                .password(Some("password")),
        )
        .unwrap();
    }

    #[test]
    fn zstd_with_camellia_cbc_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC)
                .password(Some("password")),
        )
        .unwrap();
    }

    #[test]
    fn xz_with_aes_cbc_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::XZ)
                .encryption(Encryption::Aes)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::Pbkdf2Sha256)
                .password(Some("password")),
        )
        .unwrap()
    }

    #[test]
    fn xz_with_camellia_cbc_archive() {
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::XZ)
                .encryption(Encryption::Camellia)
                .cipher_mode(CipherMode::CBC)
                .hash_algorithm(HashAlgorithm::Pbkdf2Sha256)
                .password(Some("password")),
        )
        .unwrap()
    }

    fn archive(src: &[u8], options: Options) -> io::Result<()> {
        let mut archived_temp = Vec::new();
        {
            let encoder = Encoder::new();
            let mut archive_writer = encoder.write_header(&mut archived_temp)?;
            archive_writer.start_file_with_options("test/text", options.clone())?;
            archive_writer.write_all(src)?;
            archive_writer.end_file()?;
            archive_writer.finalize()?;
        }
        let mut dist = Vec::new();
        let decoder = Decoder::new();
        let mut archive_reader = decoder.read_header(io::Cursor::new(archived_temp))?;
        let mut item = archive_reader
            .read(options.password.as_deref())
            .unwrap()
            .unwrap();
        io::copy(&mut item, &mut dist)?;
        assert_eq!(src, dist.as_slice());
        Ok(())
    }
}
