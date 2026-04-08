pub(crate) mod owner;

use super::security::{Sid, apply_security_info};
use crate::utils::str::encode_wide;
use std::io;
use std::mem::size_of;
use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_READONLY,
    FILE_BASIC_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FILE_WRITE_ATTRIBUTES, FileBasicInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
    MOVEFILE_COPY_ALLOWED, MOVEFILE_REPLACE_EXISTING, MoveFileExW, OPEN_EXISTING, READ_CONTROL,
    SetFileInformationByHandle, WRITE_DAC, WRITE_OWNER,
};
use windows::core::PCWSTR;

const MODE_SYMLINK: u16 = 0o120000;
const MODE_DIR: u16 = 0o040000;
const MODE_FILE: u16 = 0o100000;
const MODE_READ_BITS: u16 = 0o444;
const MODE_WRITE_BITS: u16 = 0o222;
const MODE_EXEC_BITS: u16 = 0o111;

#[derive(Debug)]
pub(crate) struct FileHandle(HANDLE);

impl FileHandle {
    #[inline]
    pub(crate) const fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

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
fn open_path(path: &Path, desired_access: u32, follow_symlink: bool) -> io::Result<FileHandle> {
    let path = encode_wide(path.as_os_str())?;
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_symlink {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    let handle = unsafe {
        CreateFileW(
            PCWSTR::from_raw(path.as_ptr()),
            desired_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            flags,
            None,
        )
    }?;
    Ok(FileHandle(handle))
}

#[inline]
pub(crate) fn open_read_metadata(path: &Path, follow_symlink: bool) -> io::Result<FileHandle> {
    open_path(
        path,
        (READ_CONTROL | FILE_READ_ATTRIBUTES).0,
        follow_symlink,
    )
}

#[inline]
pub(crate) fn open_write_dacl(path: &Path, follow_symlink: bool) -> io::Result<FileHandle> {
    open_path(path, (READ_CONTROL | WRITE_DAC).0, follow_symlink)
}

#[inline]
pub(crate) fn file_information(handle: HANDLE) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe { GetFileInformationByHandle(handle, &mut info) }?;
    Ok(info)
}

#[inline]
pub(crate) fn mode_from_file_information(
    path: &Path,
    info: &BY_HANDLE_FILE_INFORMATION,
    is_symlink: bool,
) -> u16 {
    let mut mode = MODE_READ_BITS;
    if info.dwFileAttributes & FILE_ATTRIBUTE_READONLY.0 == 0 {
        mode |= MODE_WRITE_BITS;
    }
    if is_symlink {
        mode |= MODE_SYMLINK;
        return mode;
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
        mode |= MODE_DIR | MODE_EXEC_BITS;
        return mode;
    }

    mode |= MODE_FILE;
    if path
        .extension()
        .and_then(|it| it.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|ext| matches!(ext.as_str(), "bat" | "cmd" | "exe"))
    {
        mode |= MODE_EXEC_BITS;
    }
    mode
}

#[inline]
pub(crate) fn lchown<U: Into<Sid>, G: Into<Sid>>(
    path: &Path,
    owner: Option<U>,
    group: Option<G>,
) -> io::Result<()> {
    let owner_sid = owner.map(Into::into);
    let group_sid = group.map(Into::into);
    let handle = open_path(path, (READ_CONTROL | WRITE_OWNER).0, false)?;
    apply_security_info(
        handle.raw(),
        owner_sid.as_ref().map(Sid::as_psid),
        group_sid.as_ref().map(Sid::as_psid),
        None,
    )
}

#[inline]
pub(crate) fn chmod(path: &Path, mode: u16) -> io::Result<()> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        let handle = open_path(
            path,
            (FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES).0,
            false,
        )?;
        let mut info = FILE_BASIC_INFO::default();
        unsafe {
            GetFileInformationByHandleEx(
                handle.raw(),
                FileBasicInfo,
                &mut info as *mut _ as _,
                size_of::<FILE_BASIC_INFO>() as u32,
            )?;
        }
        if mode & MODE_WRITE_BITS == 0 {
            info.FileAttributes |= FILE_ATTRIBUTE_READONLY.0;
        } else {
            info.FileAttributes &= !FILE_ATTRIBUTE_READONLY.0;
        }
        unsafe {
            SetFileInformationByHandle(
                handle.raw(),
                FileBasicInfo,
                &info as *const _ as _,
                size_of::<FILE_BASIC_INFO>() as u32,
            )?;
        }
        return Ok(());
    }

    let s = encode_wide(path.as_os_str())?;
    let code = unsafe { libc::wchmod(s.as_ptr() as _, mode as _) };
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::os::windows::security::SecurityDescriptor;

    #[test]
    fn file_chown() {
        let path = "chown.txt";
        std::fs::write(path, "chown").unwrap();
        let handle = open_read_metadata(path.as_ref(), true).unwrap();
        let sd = SecurityDescriptor::try_from_handle(handle).unwrap();
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
