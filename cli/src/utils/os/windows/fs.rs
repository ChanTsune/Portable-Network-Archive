pub(crate) mod owner;

use super::security::{SecurityDescriptor, Sid};
use crate::utils::str::encode_wide;
use std::io;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING,
};

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
pub(crate) fn chown<U: Into<Sid>, G: Into<Sid>>(
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

pub(crate) fn chmod(path: &Path, mode: u16) -> io::Result<()> {
    let s = encode_wide(path.as_os_str())?;
    unsafe { libc::wchmod(s.as_ptr() as _, mode as _) };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_chown() {
        let path = "chown.txt";
        std::fs::write(&path, "chown").unwrap();
        let sd = SecurityDescriptor::try_from(path.as_ref()).unwrap();
        chown::<_, Sid>(path.as_ref(), Some(sd.owner_sid().unwrap()), None).unwrap();
        chown::<Sid, _>(path.as_ref(), None, Some(sd.group_sid().unwrap())).unwrap();
        chown(
            path.as_ref(),
            Some(sd.owner_sid().unwrap()),
            Some(sd.group_sid().unwrap()),
        )
        .unwrap();
    }
}
