//! Byte-slice primitives for parsing PNA archives.

mod chunk;
mod signature;

pub use chunk::{read_chunk, skip_chunk};
pub use signature::{is_pna, read_signature};
