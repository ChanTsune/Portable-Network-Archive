mod crc;
mod read;
mod types;
mod write;

use crate::archive::{Compression, DataKind, Encryption, ItemInfo};
pub use read::ChunkReader;
use std::io;
pub use types::*;
pub use write::ChunkWriter;

pub fn create_chunk_data_ahed(major: u8, minor: u8, archive_number: u32) -> [u8; 8] {
    let mut data = [0; 8];
    data[0] = major;
    data[1] = minor;
    data[2..4].copy_from_slice(&[0, 0]);
    data[4..8].copy_from_slice(&archive_number.to_be_bytes());
    data
}

pub fn create_chunk_data_fhed(
    major: u8,
    minor: u8,
    compression: u8,
    encryption: u8,
    file_type: u8,
    name: &str,
) -> Box<[u8]> {
    let name = name.as_bytes();
    let mut data = vec![0u8; 6 + name.len()];
    data[0] = major;
    data[1] = minor;
    data[2] = compression;
    data[3] = encryption;
    data[4] = file_type;
    data[5] = 0; // null character
    data[6..].copy_from_slice(name);
    data.into_boxed_slice()
}

pub(crate) fn from_chunk_data_fhed(data: &[u8]) -> io::Result<ItemInfo> {
    Ok(ItemInfo {
        major: data[0],
        minor: data[1],
        compression: Compression::try_from(data[2])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        encryption: Encryption::try_from(data[3])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        data_kind: DataKind::try_from(data[4])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        path: String::from_utf8(data[6..].to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
    })
}

#[cfg(test)]
mod tests {
    use crate::create_chunk_data_ahed;

    #[test]
    fn ahed() {
        assert_eq!([0u8, 0, 0, 0, 0, 0, 0, 0], create_chunk_data_ahed(0, 0, 0));
        assert_eq!([1u8, 2, 0, 0, 0, 0, 0, 3], create_chunk_data_ahed(1, 2, 3));
    }
}
