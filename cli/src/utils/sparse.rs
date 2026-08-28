//! Filesystem extent detection for sparse files.
//!
//! `detect_sparse_map` returns `None` for dense files and on platforms without
//! extent queries. It may leave the file offset anywhere: the Unix scan seeks
//! the shared fd.

use pna::{DataRegion, SparseMap};
use std::{fs::File, io};

#[cfg(all(
    unix,
    not(any(
        target_os = "emscripten",
        target_os = "fuchsia",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "redox"
    ))
))]
pub(crate) fn detect_sparse_map(file: &File) -> io::Result<Option<SparseMap>> {
    use std::os::fd::AsRawFd;
    let size = file.metadata()?.len();
    if size == 0 {
        return Ok(None);
    }
    let fd = file.as_raw_fd();
    let mut regions = Vec::new();
    let mut pos = 0u64;
    while pos < size {
        let data = unsafe { libc::lseek(fd, pos as libc::off_t, libc::SEEK_DATA) };
        if data < 0 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ENXIO) {
                break;
            }
            if matches!(
                err.raw_os_error(),
                Some(libc::EINVAL) | Some(libc::EOPNOTSUPP) | Some(libc::ENOSYS)
            ) {
                return Ok(None);
            }
            return Err(err);
        }
        let hole = unsafe { libc::lseek(fd, data, libc::SEEK_HOLE) };
        if hole < 0 {
            let err = io::Error::last_os_error();
            if matches!(
                err.raw_os_error(),
                Some(libc::EINVAL) | Some(libc::EOPNOTSUPP) | Some(libc::ENOSYS)
            ) {
                return Ok(None);
            }
            return Err(err);
        }
        let start =
            u64::try_from(data).map_err(|_| io::Error::other("negative SEEK_DATA offset"))?;
        let end = u64::try_from(hole)
            .map_err(|_| io::Error::other("negative SEEK_HOLE offset"))?
            .min(size);
        if end < start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SEEK_HOLE before SEEK_DATA",
            ));
        }
        if end > start {
            regions.push(DataRegion::new(start, end - start));
        }
        if end <= pos {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sparse extent scan made no progress",
            ));
        }
        pos = end;
    }
    if regions.len() == 1 && regions[0].offset() == 0 && regions[0].size() == size {
        Ok(None)
    } else {
        SparseMap::try_new(size, regions).map(Some)
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::{ffi::c_void, mem::size_of, os::windows::io::AsRawHandle, ptr};

    const FSCTL_SET_SPARSE: u32 = 0x0009_00c4;
    const FSCTL_QUERY_ALLOCATED_RANGES: u32 = 0x0009_40cf;
    const ERROR_INVALID_FUNCTION: i32 = 1;
    const ERROR_NOT_SUPPORTED: i32 = 50;
    const ERROR_MORE_DATA: i32 = 234;
    const MAX_RANGES: usize = 1_048_576;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct AllocatedRange {
        offset: i64,
        length: i64,
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn DeviceIoControl(
            device: *mut c_void,
            control: u32,
            input: *mut c_void,
            input_len: u32,
            output: *mut c_void,
            output_len: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
    }

    pub(super) fn detect(file: &File) -> io::Result<Option<SparseMap>> {
        let size = file.metadata()?.len();
        if size == 0 || size > i64::MAX as u64 {
            return Ok(None);
        }
        let input = AllocatedRange {
            offset: 0,
            length: size as i64,
        };
        let mut capacity = 64usize;
        loop {
            if capacity > MAX_RANGES {
                log::warn!(
                    "File has more than {MAX_RANGES} allocated ranges; storing it without a sparse map"
                );
                return Ok(None);
            }
            let mut output = vec![AllocatedRange::default(); capacity];
            let mut returned = 0u32;
            let ok = unsafe {
                DeviceIoControl(
                    file.as_raw_handle().cast(),
                    FSCTL_QUERY_ALLOCATED_RANGES,
                    (&input as *const AllocatedRange).cast_mut().cast(),
                    size_of::<AllocatedRange>() as u32,
                    output.as_mut_ptr().cast(),
                    (output.len() * size_of::<AllocatedRange>()) as u32,
                    &mut returned,
                    ptr::null_mut(),
                )
            };
            if ok == 0 {
                let err = io::Error::last_os_error();
                return match err.raw_os_error() {
                    Some(ERROR_MORE_DATA) => {
                        capacity *= 2;
                        continue;
                    }
                    Some(ERROR_INVALID_FUNCTION) | Some(ERROR_NOT_SUPPORTED) => Ok(None),
                    _ => Err(err),
                };
            }
            if !(returned as usize).is_multiple_of(size_of::<AllocatedRange>()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid allocated-range result length",
                ));
            }
            output.truncate(returned as usize / size_of::<AllocatedRange>());
            let mut regions = Vec::with_capacity(output.len());
            for range in output {
                if range.offset < 0 || range.length < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "negative allocated range",
                    ));
                }
                let offset = range.offset as u64;
                if offset >= size {
                    continue;
                }
                // Allocation is reported in cluster units and may extend past EOF.
                let length = (range.length as u64).min(size - offset);
                if length != 0 {
                    regions.push(DataRegion::new(offset, length));
                }
            }
            if regions.len() == 1 && regions[0].offset() == 0 && regions[0].size() == size {
                return Ok(None);
            }
            return SparseMap::try_new(size, regions).map(Some);
        }
    }

    pub(super) fn mark(file: &File) -> io::Result<()> {
        let mut returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                file.as_raw_handle().cast(),
                FSCTL_SET_SPARSE,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
pub(crate) fn detect_sparse_map(file: &File) -> io::Result<Option<SparseMap>> {
    windows_impl::detect(file)
}

#[cfg(any(
    not(any(unix, windows)),
    target_os = "emscripten",
    target_os = "fuchsia",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox"
))]
pub(crate) fn detect_sparse_map(_file: &File) -> io::Result<Option<SparseMap>> {
    Ok(None)
}

#[cfg(windows)]
pub(crate) fn mark_sparse(file: &File) -> io::Result<()> {
    windows_impl::mark(file)
}

#[cfg(not(windows))]
pub(crate) fn mark_sparse(_file: &File) -> io::Result<()> {
    Ok(())
}
