//! Stream cipher reader and writer implementations.

mod read;
mod write;

pub(crate) use read::StreamCipherReader;
pub(crate) use write::StreamCipherWriter;
