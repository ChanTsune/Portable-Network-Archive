mod crc;
mod read;
mod types;
mod write;

pub use read::ChunkReader;
pub use types::*;
pub use write::ChunkWriter;

pub(crate) fn create_chunk_data_ahed(major: u8, minor: u8, archive_number: u32) -> [u8; 8] {
    let mut data = [0; 8];
    data[0] = major;
    data[1] = minor;
    data[2..4].copy_from_slice(&[0, 0]);
    data[4..8].copy_from_slice(&archive_number.to_be_bytes());
    data
}

pub(crate) fn create_chunk_data_fhed(
    major: u8,
    minor: u8,
    compression: u8,
    encryption: u8,
    cipher_mode: u8,
    file_type: u8,
    name: &str,
) -> Box<[u8]> {
    let name = name.as_bytes();
    let mut data = vec![0u8; 6 + name.len()];
    data[0] = major;
    data[1] = minor;
    data[2] = file_type;
    data[3] = compression;
    data[4] = encryption;
    data[5] = cipher_mode;
    data[6..].copy_from_slice(name);
    data.into_boxed_slice()
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
