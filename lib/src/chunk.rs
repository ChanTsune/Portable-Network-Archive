mod types;

pub(crate) use types::*;

pub fn create_chunk_data_ahed(major: u8, minor: u8, archive_number: u32) -> [u8; 8] {
    let mut data = [0; 8];
    data[0] = major;
    data[1] = minor;
    data[2..4].copy_from_slice(&[0, 0]);
    data[4..8].copy_from_slice(&archive_number.to_be_bytes());
    data
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
