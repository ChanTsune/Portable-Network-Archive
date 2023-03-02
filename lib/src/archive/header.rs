use std::io::{self, Write};
/// The magic number of Portable-Network-Archive
pub const PNA_HEADER: &[u8; 8] = b"\x89PNA\r\n\x1A\n";

#[inline]
pub(crate) fn write_pna_header<W: Write>(writer: &mut W) -> io::Result<usize> {
    writer.write_all(PNA_HEADER)?;
    Ok(PNA_HEADER.len())
}
