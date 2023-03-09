mod name;
mod options;
mod write;

use crate::ChunkType;
pub use name::*;
pub use options::*;
use std::io::{self, Read};
pub(crate) use write::*;

pub struct EntryHeader {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) data_kind: DataKind,
    pub(crate) compression: Compression,
    pub(crate) encryption: Encryption,
    pub(crate) cipher_mode: CipherMode,
    pub(crate) path: ItemName,
}

/// Chunks from `FHED` to `FEND`, containing `FHED` and `FEND`
pub(crate) struct RawEntry {
    pub(crate) chunks: Vec<(ChunkType, Vec<u8>)>,
}

pub struct Entry {
    pub(crate) header: EntryHeader,
    pub(crate) reader: Box<dyn Read + Sync + Send>,
}

impl Entry {
    pub fn reader(self) -> io::Result<impl Read + Sync + Send> {
        Ok(self.reader)
    }

    pub fn path(&self) -> &str {
        self.header.path.as_ref()
    }

    pub fn kind(&self) -> DataKind {
        self.header.data_kind
    }
}
