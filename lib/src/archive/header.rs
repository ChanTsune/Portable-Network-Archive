use std::io;

/// The magic number of Portable-Network-Archive
pub const PNA_HEADER: &[u8; 8] = b"\x89PNA\r\n\x1A\n";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ArchiveHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) archive_number: u32,
}

impl ArchiveHeader {
    #[inline]
    pub(crate) const fn new(major: u8, minor: u8, archive_number: u32) -> Self {
        Self {
            major,
            minor,
            archive_number,
        }
    }

    pub(crate) const fn to_bytes(&self) -> [u8; 8] {
        let archive_number = self.archive_number.to_be_bytes();
        [
            self.major,
            self.minor,
            0,
            0,
            archive_number[0],
            archive_number[1],
            archive_number[2],
            archive_number[3],
        ]
    }

    #[inline]
    pub(crate) const fn from_bytes(bytes: &[u8; 8]) -> Self {
        let major = bytes[0];
        let minor = bytes[1];
        // NOTE: ignore 2 bytes currently unused.
        let archive_number = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        Self::new(major, minor, archive_number)
    }

    #[inline]
    pub(crate) fn try_from_bytes(bytes: &[u8]) -> io::Result<Self> {
        Ok(Self::from_bytes(bytes.try_into().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, e)
        })?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn header_to_bytes() {
        assert_eq!(
            [0u8, 0, 0, 0, 0, 0, 0, 0],
            ArchiveHeader::new(0, 0, 0).to_bytes()
        );
        assert_eq!(
            [1u8, 2, 0, 0, 0, 0, 0, 3],
            ArchiveHeader::new(1, 2, 3).to_bytes()
        );
    }

    #[test]
    fn header_from_bytes() {
        assert_eq!(
            ArchiveHeader::from_bytes(&[0u8, 0, 0, 0, 0, 0, 0, 0]),
            ArchiveHeader::new(0, 0, 0)
        );
        assert_eq!(
            ArchiveHeader::from_bytes(&[1u8, 2, 0, 0, 0, 0, 0, 3]),
            ArchiveHeader::new(1, 2, 3)
        );
    }

    #[test]
    fn header_try_from_bytes() {
        assert!(ArchiveHeader::try_from_bytes(&[0u8; 7]).is_err());
        assert!(ArchiveHeader::try_from_bytes(&[0u8; 8]).is_ok());
        assert!(ArchiveHeader::try_from_bytes(&[0u8; 9]).is_err());
    }

    #[test]
    fn header_to_from_bytes() {
        let bytes = [1u8, 2, 0, 0, 0, 0, 0, 3];
        assert_eq!(
            ArchiveHeader::from_bytes(&bytes),
            ArchiveHeader::from_bytes(&ArchiveHeader::from_bytes(&bytes).to_bytes()),
        );
    }
}
