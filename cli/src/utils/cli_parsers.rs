//! Clap value parsers that validate CLI strings against PNA's bounded
//! identifier types so out-of-range input is rejected by the argument parser
//! before reaching the rest of the program.

#[inline]
pub(crate) fn parse_uname(s: &str) -> Result<pna::UserName, pna::LengthExceeded> {
    pna::UserName::try_from(s)
}

#[inline]
pub(crate) fn parse_gname(s: &str) -> Result<pna::GroupName, pna::LengthExceeded> {
    pna::GroupName::try_from(s)
}

#[inline]
pub(crate) fn parse_xattr_name(s: &str) -> Result<pna::XattrName, pna::LengthExceeded> {
    pna::XattrName::try_from(s)
}
