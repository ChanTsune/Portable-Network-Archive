use std::{io, path::Path};

#[allow(unused_imports)]
use std::ffi::OsStr;
#[allow(unused_imports)]
use std::os::windows::ffi::OsStrExt;
use std::iter::once; // For null termination in wide strings

use windows_sys::Win32::Storage::FileSystem::{
    GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_READONLY,
    INVALID_FILE_ATTRIBUTES,
};
use windows_sys::Win32::Foundation::FALSE; // For checking SetFileAttributesW result

// Returns the DWORD value of file attributes.
pub fn get_file_attributes(path: &Path) -> io::Result<u32> {
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: The path_wide pointer is valid and points to a null-terminated UTF-16 string.
    // The GetFileAttributesW function is a standard Windows API call.
    let attributes = unsafe { GetFileAttributesW(path_wide.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        Err(io::Error::last_os_error())
    } else {
        Ok(attributes)
    }
}

// Sets the file attributes using the provided DWORD value.
pub fn set_file_attributes(path: &Path, attributes: u32) -> io::Result<()> {
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();
    // SAFETY: The path_wide pointer is valid and points to a null-terminated UTF-16 string.
    // The SetFileAttributesW function is a standard Windows API call.
    let success = unsafe { SetFileAttributesW(path_wide.as_ptr(), attributes) };
    if success == FALSE {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_ARCHIVE;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_get_file_attributes_readonly() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_get_readonly.txt");

        // Create a file
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();
        drop(file); // Close the file

        // Make the file readonly using std::fs, then check with our function
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file_path, perms).unwrap();

        let attributes = get_file_attributes(&file_path).unwrap();
        assert_ne!(
            attributes & FILE_ATTRIBUTE_READONLY,
            0,
            "FILE_ATTRIBUTE_READONLY should be set"
        );

        // Clean up by making it writable again
        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_readonly(false);
        fs::set_permissions(&file_path, perms).unwrap();
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_get_file_attributes_hidden() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_get_hidden.txt");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "Hello, world!").unwrap();
        drop(file);

        // For this test, we assume the file is NOT hidden by default.
        // Setting hidden attribute is tested in set_file_attributes tests.
        let attributes = get_file_attributes(&file_path).unwrap();
        assert_eq!(
            attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN,
            0,
            "FILE_ATTRIBUTE_HIDDEN should not be set by default"
        );

        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_get_file_attributes_non_existent_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent_get_file.txt");

        let result = get_file_attributes(&file_path);
        assert!(result.is_err());
        if let Err(e) = result {
            let raw_os_error = e.raw_os_error();
            assert!(
                raw_os_error == Some(2) || raw_os_error == Some(3),
                "Unexpected OS error for non-existent file: {:?}, expected 2 (FileNotFound) or 3 (PathNotFound)",
                raw_os_error
            );
        }
    }

    #[test]
    fn test_set_and_get_file_attributes_readonly() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_set_readonly.txt");

        fs::File::create(&file_path).unwrap().write_all(b"test").unwrap();

        // Set READONLY attribute
        set_file_attributes(&file_path, FILE_ATTRIBUTE_READONLY).unwrap();
        let attributes = get_file_attributes(&file_path).unwrap();
        assert_ne!(
            attributes & FILE_ATTRIBUTE_READONLY,
            0,
            "FILE_ATTRIBUTE_READONLY should be set after set_file_attributes"
        );

        // Clear READONLY attribute by setting to NORMAL (most common way to clear)
        // Note: To be precise, one should get current attributes, XOR the specific bit, then set.
        // But for testing readonly, setting to NORMAL is often sufficient if other bits aren't critical.
        // Or, ensure the file starts with NORMAL/ARCHIVE and then add/remove readonly.
        // For a clean test, let's fetch, modify, and set.
        let current_attributes = get_file_attributes(&file_path).unwrap();
        set_file_attributes(&file_path, current_attributes & !FILE_ATTRIBUTE_READONLY).unwrap();
        let attributes_after_clear = get_file_attributes(&file_path).unwrap();
        assert_eq!(
            attributes_after_clear & FILE_ATTRIBUTE_READONLY,
            0,
            "FILE_ATTRIBUTE_READONLY should be cleared"
        );

        // Attempt to write to the file should succeed now
        fs::OpenOptions::new().write(true).open(&file_path).unwrap().write_all(b"more").unwrap();


        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_set_file_attributes_multiple() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_set_multiple.txt");
        fs::File::create(&file_path).unwrap().write_all(b"test").unwrap();

        let initial_attributes = get_file_attributes(&file_path).unwrap();
        // Ensure it's not readonly or hidden initially for a clean test
        assert_eq!(initial_attributes & FILE_ATTRIBUTE_READONLY, 0);
        assert_eq!(initial_attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN, 0);


        let new_attrs = FILE_ATTRIBUTE_READONLY | windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
        set_file_attributes(&file_path, new_attrs).unwrap();

        let attributes = get_file_attributes(&file_path).unwrap();
        assert_ne!(
            attributes & FILE_ATTRIBUTE_READONLY,
            0,
            "FILE_ATTRIBUTE_READONLY should be set"
        );
        assert_ne!(
            attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN,
            0,
            "FILE_ATTRIBUTE_HIDDEN should be set"
        );

        // Clear attributes back to something normal (e.g., ARCHIVE or NORMAL)
        // Ensure we clear the specific bits we set, respecting other possible bits like ARCHIVE
        let final_attrs = (attributes & !(FILE_ATTRIBUTE_READONLY | windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN)) | FILE_ATTRIBUTE_NORMAL;
        set_file_attributes(&file_path, final_attrs).unwrap();
        let cleared_attributes = get_file_attributes(&file_path).unwrap();
        assert_eq!(
            cleared_attributes & FILE_ATTRIBUTE_READONLY,
            0,
            "FILE_ATTRIBUTE_READONLY should be cleared"
        );
        assert_eq!(
            cleared_attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN,
            0,
            "FILE_ATTRIBUTE_HIDDEN should be cleared"
        );
        // It might have FILE_ATTRIBUTE_NORMAL or FILE_ATTRIBUTE_ARCHIVE now
        assert!(cleared_attributes == FILE_ATTRIBUTE_NORMAL || cleared_attributes & FILE_ATTRIBUTE_ARCHIVE != 0);


        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_set_file_attributes_non_existent_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("non_existent_set_file.txt");

        let result = set_file_attributes(&file_path, FILE_ATTRIBUTE_READONLY);
        assert!(result.is_err());
        if let Err(e) = result {
            let raw_os_error = e.raw_os_error();
             assert!(
                raw_os_error == Some(2) || raw_os_error == Some(3), // ERROR_FILE_NOT_FOUND or ERROR_PATH_NOT_FOUND
                "Unexpected OS error for non-existent file: {:?}, expected 2 or 3",
                raw_os_error
            );
        }
    }

    // Helper to ensure a file is writable before attempting to delete, useful for cleanup
    fn ensure_writable_and_delete(path: &Path) {
        if path.exists() {
            if let Ok(attrs) = get_file_attributes(path) {
                if attrs & FILE_ATTRIBUTE_READONLY != 0 {
                    let _ = set_file_attributes(path, attrs & !FILE_ATTRIBUTE_READONLY);
                }
            }
            let _ = fs::remove_file(path);
        }
    }

    // Test setting FILE_ATTRIBUTE_NORMAL
    #[test]
    fn test_set_file_attribute_normal() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_set_normal.txt");
        fs::File::create(&file_path).unwrap().write_all(b"test").unwrap();

        // Set some attributes first
        set_file_attributes(&file_path, FILE_ATTRIBUTE_READONLY | windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN).unwrap();
        let attrs_before_normal = get_file_attributes(&file_path).unwrap();
        assert_ne!(attrs_before_normal & FILE_ATTRIBUTE_READONLY, 0);
        assert_ne!(attrs_before_normal & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN, 0);


        // Set FILE_ATTRIBUTE_NORMAL. This attribute is only valid if used alone.
        set_file_attributes(&file_path, FILE_ATTRIBUTE_NORMAL).unwrap();
        let attributes = get_file_attributes(&file_path).unwrap();
        
        // FILE_ATTRIBUTE_NORMAL means no other attributes are set.
        // However, the system may still report FILE_ATTRIBUTE_ARCHIVE.
        // So, we check that READONLY and HIDDEN are not set.
        assert_eq!(attributes & FILE_ATTRIBUTE_READONLY, 0, "READONLY should be cleared by NORMAL");
        assert_eq!(attributes & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN, 0, "HIDDEN should be cleared by NORMAL");
        // The resulting attributes might be FILE_ATTRIBUTE_NORMAL or FILE_ATTRIBUTE_NORMAL | FILE_ATTRIBUTE_ARCHIVE
        // or just FILE_ATTRIBUTE_ARCHIVE on some systems if the file was modified.
        // Let's check that it's primarily normal or archive, and definitely not the ones we cleared.
        let is_normal_or_archive = attributes == FILE_ATTRIBUTE_NORMAL || attributes == (FILE_ATTRIBUTE_NORMAL | FILE_ATTRIBUTE_ARCHIVE) || attributes == FILE_ATTRIBUTE_ARCHIVE;
        assert!(is_normal_or_archive, "Attributes after setting NORMAL should be NORMAL or ARCHIVE, got: {:b}", attributes);


        ensure_writable_and_delete(&file_path);
    }
}
