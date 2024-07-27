use crate::{archive::ArchiveHeader, chunk::ChunkReader, Archive, Chunk, ChunkType, PNA_HEADER};
use std::io;

fn read_header_from_slice(bytes: &[u8]) -> io::Result<&[u8]> {
    let (header, body) = bytes
        .split_at_checked(PNA_HEADER.len())
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    if &header != PNA_HEADER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "It's not PNA"));
    }
    Ok(body)
}

impl<'d> Archive<&'d [u8]> {
    pub fn read_header_from_slice(bytes: &'d [u8]) -> io::Result<Self> {
        let bytes = read_header_from_slice(bytes)?;
        let mut chunk_reader = ChunkReader::from(bytes);
        let chunk = chunk_reader.read_chunk_from_slice()?;
        if chunk.ty != ChunkType::AHED {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected Chunk `{}`", chunk.ty),
            ));
        }
        let header = ArchiveHeader::try_from_bytes(chunk.data())?;
        Ok(Self::new(chunk_reader.r, header))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_header() {
        let result = read_header_from_slice(PNA_HEADER).unwrap();
        assert_eq!(result, &[]);
    }

    #[test]
    fn decode() {
        let bytes = include_bytes!("../../../../resources/test/zstd.pna");
        let mut archive = Archive::read_header_from_slice(bytes).unwrap();
        let mut entries = archive.entries();
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_some());
        assert!(entries.next().is_none());
    }
}
