//! Byte-slice primitives for parsing PNA archives.

mod chunk;
mod chunks;
mod signature;

pub use chunk::{read_chunk, skip_chunk};
pub use chunks::read_chunks;
pub use signature::{is_pna, read_signature};
