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
    use std::io;

    use super::{Compression, Decoder, Encoder, Options};

    #[test]
    fn store_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::No),
        )
    }

    #[test]
    fn deflate_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::Deflate),
        )
    }

    #[test]
    fn zstd_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::ZStandard),
        )
    }

    #[test]
    fn xz_archive() {
        archive(
            b"src data bytes",
            Options::default().compression(Compression::XZ),
        )
    }

    fn archive(src: &[u8], options: Options) {
        let mut archived_temp = Vec::new();
        {
            let encoder = Encoder::new();
            let mut archive_writer = encoder.write_header(&mut archived_temp).unwrap();
            archive_writer
                .start_file_with_options("test/text", options)
                .unwrap();
            archive_writer.write_all(src).unwrap();
            archive_writer.end_file().unwrap();
            archive_writer.finalize().unwrap();
        }
        let mut dist = Vec::new();
        let decoder = Decoder::new();
        let mut archive_reader = decoder.read_header(io::Cursor::new(archived_temp)).unwrap();
        let mut item = archive_reader.read().unwrap().unwrap();
        io::copy(&mut item, &mut dist).unwrap();
        assert_eq!(src, dist.as_slice());
    }
}
