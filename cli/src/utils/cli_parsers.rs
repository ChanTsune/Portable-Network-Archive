//! Clap value parser that validates the CLI extended-attribute name against
//! PNA's bounded `XattrName` type so an over-long name is rejected by the
//! argument parser before reaching the rest of the program.

#[inline]
pub(crate) fn parse_xattr_name(s: &str) -> Result<pna::XattrName, pna::LengthExceeded> {
    pna::XattrName::try_from(s)
}
