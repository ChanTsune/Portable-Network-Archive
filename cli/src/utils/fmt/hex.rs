use std::fmt;

#[inline]
pub(crate) fn display(value: &[u8]) -> HexDisplay<'_> {
    HexDisplay(value)
}

pub(crate) struct HexDisplay<'a>(&'a [u8]);

impl fmt::Display for HexDisplay<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in self.0 {
            write!(f, "{i:02x}")?;
        }
        Ok(())
    }
}
