use std::io::{self, Cursor, Read};

#[derive(Copy, Clone)]
pub enum Compression {
    No = 0,
    Deflate = 1,
    ZStandard = 2,
    XZ = 4,
}

#[derive(Copy, Clone)]
pub enum Encryption {
    No = 0,
    AES = 1,
    Camellia = 2,
}

#[derive(Copy, Clone)]
pub enum DataKind {
    File = 0,
    Directory = 1,
    SymbolicLink = 2,
    HardLink = 3,
}

pub struct Options {
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) password: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            compression: Compression::No,
            encryption: Encryption::No,
            password: None,
        }
    }
}

pub struct ItemInfo {
    major: u8,
    minor: u8,
    compression: Compression,
    encryption: Encryption,
    data_kind: DataKind,
    path: String,
}

pub struct Item {
    info: ItemInfo,
    reader: Cursor<Vec<u8>>,
}

impl Read for Item {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}
