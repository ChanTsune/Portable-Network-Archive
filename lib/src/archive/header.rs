use std::io::{self, Read};

/// The magic number of Portable-Network-Archive
pub const PNA_HEADER: &[u8; 8] = b"\x89PNA\r\n\x1A\n";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ArchiveHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) archive_number: u32,
}

impl ArchiveHeader {
    pub(crate) const fn new(major: u8, minor: u8, archive_number: u32) -> Self {
        Self {
            major,
            minor,
            archive_number,
        }
    }

    pub(crate) const fn to_bytes(&self) -> [u8; 8] {
        let mut data = [0; 8];
        data[0] = self.major;
        data[1] = self.minor;
        data[2] = 0;
        data[3] = 0;
        let byte = self.archive_number.to_be_bytes();
        data[4] = byte[0];
        data[5] = byte[1];
        data[6] = byte[2];
        data[7] = byte[3];
        data
    }

    pub(crate) fn try_from_bytes(mut bytes: &[u8]) -> io::Result<Self> {
        let major = {
            let mut buf = [0; 1];
            bytes.read_exact(&mut buf)?;
            buf[0]
        };
        let minor = {
            let mut buf = [0; 1];
            bytes.read_exact(&mut buf)?;
            buf[0]
        };

        // NOTE: ignore 2bytes currently unused.
        bytes.read_exact(&mut [0; 2])?;

        let archive_number = {
            let mut buf = [0; 4];
            bytes.read_exact(&mut buf)?;
            u32::from_be_bytes(buf)
        };
        Ok(Self::new(major, minor, archive_number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            ArchiveHeader::try_from_bytes(&[0u8, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
            ArchiveHeader::new(0, 0, 0)
        );
        assert_eq!(
            ArchiveHeader::try_from_bytes(&[1u8, 2, 0, 0, 0, 0, 0, 3]).unwrap(),
            ArchiveHeader::new(1, 2, 3)
        );
    }
}
