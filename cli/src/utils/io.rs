use std::io;

pub(crate) fn is_pna<R: io::Read>(mut reader: R) -> io::Result<bool> {
    let mut buf = [0u8; pna::PNA_HEADER.len()];
    reader.read_exact(&mut buf)?;
    Ok(buf == *pna::PNA_HEADER)
}
