use std::borrow::Cow;
use std::ffi::OsStr;
#[cfg(unix)]
use std::{
    os::unix::ffi::OsStrExt,
    str::{self, Utf8Error},
};
#[cfg(windows)]
use std::{os::windows::ffi::OsStrExt, string::FromUtf16Error};

#[cfg(unix)]
pub(crate) fn try_to_string(s: &OsStr) -> Result<Cow<str>, Utf8Error> {
    str::from_utf8(s.as_bytes()).map(Cow::from)
}

#[cfg(windows)]
pub(crate) fn try_to_string(s: &OsStr) -> Result<Cow<str>, FromUtf16Error> {
    String::from_utf16(&s.encode_wide().collect::<Vec<_>>()).map(Cow::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn safe_chars() {
        assert_eq!(try_to_string("".as_ref()).unwrap(), Cow::from(""));
    }
}
