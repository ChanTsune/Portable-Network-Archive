pub(crate) mod owner;

use super::security::{SecurityDescriptor, Sid};
use crate::utils::str::encode_wide;
use std::io;
use std::path::Path;
use windows::Win32::Storage::FileSystem::{
    MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING, MoveFileExW,
};
use windows::core::PCWSTR;

#[inline]
pub(crate) fn move_file(src: &std::ffi::OsStr, dist: &std::ffi::OsStr) -> io::Result<()> {
    unsafe {
        MoveFileExW(
            PCWSTR::from_raw(encode_wide(src)?.as_ptr()),
            PCWSTR::from_raw(encode_wide(dist)?.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED,
        )
    }
    .map_err(Into::into)
}

#[inline]
pub(crate) fn lchown<U: Into<Sid>, G: Into<Sid>>(
    path: &Path,
    owner: Option<U>,
    group: Option<G>,
) -> io::Result<()> {
    let sd = SecurityDescriptor::try_from(path)?;
    sd.apply(
        owner.and_then(|it| it.into().to_psid().ok()),
        group.and_then(|it| it.into().to_psid().ok()),
        None,
    )
}

#[inline]
pub(crate) fn chmod(path: &Path, mode: u16) -> io::Result<()> {
    let s = encode_wide(path.as_os_str())?;
    let code = unsafe { libc::wchmod(s.as_ptr() as _, mode as _) };
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(crate) fn stat(path: *const libc::wchar_t) -> io::Result<libc::stat> {
    let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
    let code = unsafe { libc::wstat(path, &mut stat) };
    if code == 0 {
        Ok(stat)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_chown() {
        let path = "chown.txt";
        std::fs::write(&path, "chown").unwrap();
        let sd = SecurityDescriptor::try_from(path.as_ref()).unwrap();
        lchown::<_, Sid>(path.as_ref(), Some(sd.owner_sid().unwrap()), None).unwrap();
        lchown::<Sid, _>(path.as_ref(), None, Some(sd.group_sid().unwrap())).unwrap();
        lchown(
            path.as_ref(),
            Some(sd.owner_sid().unwrap()),
            Some(sd.group_sid().unwrap()),
        )
        .unwrap();
    }
}
