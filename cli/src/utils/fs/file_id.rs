use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use winapi::um::fileapi::GetFileInformationByHandle;
#[cfg(windows)]
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use winapi::um::minwinbase::BY_HANDLE_FILE_INFORMATION;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileId(u64, u64); // (device, inode)

fn get_file_id(path: &Path) -> io::Result<FileId> {
    let file = File::open(path)?;
    let meta = file.metadata()?;

    #[cfg(unix)]
    {
        let dev = meta.dev();
        let ino = meta.ino();
        Ok(FileId(dev, ino))
    }

    #[cfg(windows)]
    unsafe {
        let handle = file.as_raw_handle();
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }

        let mut info: BY_HANDLE_FILE_INFORMATION = std::mem::zeroed();
        if GetFileInformationByHandle(handle, &mut info) == 0 {
            return Err(io::Error::last_os_error());
        }

        let volume = info.dwVolumeSerialNumber as u64;
        let index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
        Ok(FileId(volume, index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hardlink() -> io::Result<()> {
        let mut seen: HashSet<FileId> = HashSet::new();
        let files = vec!["file1.txt", "file2.txt", "file3.txt"];

        for fname in files {
            let path = Path::new(fname);
            match get_file_id(path) {
                Ok(fid) => {
                    if seen.contains(&fid) {
                        println!("{} is a hardlink", fname);
                    } else {
                        println!("{} is new (recorded)", fname);
                        seen.insert(fid);
                    }
                }
                Err(e) => eprintln!("Failed to stat {}: {}", fname, e),
            }
        }
        Ok(())
    }
}
