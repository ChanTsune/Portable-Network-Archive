use std::fmt::{self, Write};

#[inline]
pub(crate) fn display(value: &[u8]) -> HexDisplay<'_> {
    HexDisplay(value)
}

pub(crate) struct HexDisplay<'a>(&'a [u8]);

impl fmt::Display for HexDisplay<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX_CHARS: &[u8] = b"0123456789abcdef";
        for &byte in self.0 {
            f.write_char(HEX_CHARS[(byte >> 4) as usize] as char)?;
            f.write_char(HEX_CHARS[(byte & 0x0f) as usize] as char)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_string() {
        assert_eq!("012345", display(&[0x01, 0x23, 0x45]).to_string());
    }
}
