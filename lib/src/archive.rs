mod header;
mod item;
mod read;
mod write;

pub use header::PNA_HEADER;
pub use item::{Compression, DataKind, Encryption, Item, ItemInfo, Options};
pub use read::{ArchiveReader, Decoder};
pub use write::{ArchiveWriter, Encoder};

#[cfg(test)]
mod tests {
    use crate::archive::Encryption;
    use std::io;

    use super::{Compression, Decoder, Encoder, Options};

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
    fn zstd_with_aes_archive() {
        let a = [0, 1, 2, 3];
        assert_eq!(a[0..2], [0, 1]);
        assert_eq!(a[2..], [2, 3]);
        archive(
            b"plain text",
            Options::default()
                .compression(Compression::ZStandard)
                .encryption(Encryption::Aes)
                .password(Some("password".to_string())),
        )
        .unwrap();
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
