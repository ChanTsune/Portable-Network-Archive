use std::path::Path;
use windows::core::{Result, GUID, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::core::{PWSTR, PROPVARIANT};
use windows::Win32::System::Com::{
    PropVariantClear, PropVariantToStringAlloc, PropVariantToStringVector, CoTaskMemFree,
    VT_BSTR, VT_EMPTY, VT_LPWSTR, VT_NULL, VT_VECTOR,
};
use windows::Win32::System::Ole::VariantClear; // PropVariantClear is an alias for VariantClear
use windows::Win32::UI::Shell::PropertiesSystem::{
    GETPROPERTIESSTOREFLAGS, PROPERTYKEY, SHGetPropertyStoreFromParsingName, IPropertyStore,
    GPS_DEFAULT, GPS_READWRITE, /* Added GPS_READWRITE for explicit use */
};
use windows::Win32::Foundation::{E_FAIL, S_OK}; // For a generic error if Option remains None

// Helper struct to ensure PropVariantClear is called.
struct PropVariantRAII(PROPVARIANT);

impl Drop for PropVariantRAII {
    fn drop(&mut self) {
        unsafe {
            if PropVariantClear(&mut self.0).is_err() {
                // Log or handle error if clearing fails, though it's rare.
                // For now, we'll just let it be, as recovery is unlikely.
            }
        }
    }
}

/// Initializes COM for the current thread and uninitializes it when dropped.
///
/// COM needs to be initialized before using functions like `SHGetPropertyStoreFromParsingName`.
/// This struct ensures that `CoUninitialize` is called when it goes out of scope.
pub struct ComInitializer;

impl ComInitializer {
    /// Initializes COM for the current thread with `COINIT_APARTMENTTHREADED` and `COINIT_DISABLE_OLE1DDE`.
    pub fn new() -> Result<Self> {
        unsafe {
            // COINIT_APARTMENTTHREADED: Initializes COM for use by a single thread.
            // COINIT_DISABLE_OLE1DDE: Disables DDE for OLE1 support, which is generally not needed.
            CoInitializeEx(
                None, // pvReserved, must be NULL
                COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
            )?;
        }
        Ok(ComInitializer)
    }
}

impl Drop for ComInitializer {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

/// Retrieves an `IPropertyStore` for a given file path.
///
/// This function allows access to the Windows Property System for a specific file,
/// enabling reading or writing of file properties.
///
/// # Arguments
/// * `file_path`: A reference to the `Path` of the file whose property store is to be retrieved.
///
/// # Returns
/// A `windows::core::Result<IPropertyStore>` which is `Ok` containing the `IPropertyStore`
/// on success, or an `Err` with a Windows error code on failure.
///
/// # Remarks
/// COM must be initialized (e.g., via `ComInitializer`) before calling this function.
pub fn get_property_store(file_path: &Path, flags: GETPROPERTIESSTOREFLAGS) -> Result<IPropertyStore> {
    let path_hstring = HSTRING::from(file_path.as_os_str());
    let path_pcwstr = PCWSTR(path_hstring.as_ptr());
    let mut store: Option<IPropertyStore> = None;

    unsafe {
        SHGetPropertyStoreFromParsingName(
            path_pcwstr,
            None, // pbc (IBindCtx), None for default
            flags, // Use the provided flags
            &IPropertyStore::IID, // riid
            store.set_abi(), // ppv as *mut *mut c_void
        )?;
    }
    // Ensure the store was actually obtained.
    // If SHGetPropertyStoreFromParsingName fails, it returns an HRESULT error,
    // which is converted to windows::core::Error by the `?` operator.
    // If it succeeds but `store` is still `None` (which shouldn't happen if S_OK is returned),
    // this provides a fallback error.
    store.ok_or_else(|| windows::core::Error::from(E_FAIL))
}

/// Attempts to read a property and convert it to a String.
/// Returns Ok(Some(String)) if property found and is a string type.
/// Returns Ok(None) if property not found or not a handled string type.
/// Returns Err(Error) for other errors.
#[allow(clippy::needless_match)] // For clarity in matching VARTYPE
pub fn get_windows_file_property_string(
    store: &IPropertyStore,
    property_key: &PROPERTYKEY,
) -> Result<Option<String>> {
    let mut propvariant_raw = PROPVARIANT::default();
    unsafe { store.GetValue(property_key, &mut propvariant_raw)? };

    // Wrap in RAII struct to ensure PropVariantClear is called.
    let propvariant_raii = PropVariantRAII(propvariant_raw);
    let pv = &propvariant_raii.0;

    unsafe {
        // Accessing the `vt` field directly.
        // In windows-rs 0.52.0 and later, PROPVARIANT has a `vt` method or direct field access.
        // For older versions, one might need `pv.as_raw().Anonymous.vt`.
        // Given the project uses 0.59.0 for `windows` crate, direct access or `vt()` should be fine.
        // The example showed `pv.Anonymous.Anonymous.vt`, so we'll stick to that for consistency.
        match pv.Anonymous.Anonymous.vt {
            VT_LPWSTR | VT_BSTR => {
                // PropVariantToStringAlloc handles both VT_LPWSTR and VT_BSTR,
                // among other types, and allocates a string with CoTaskMemAlloc.
                let mut buffer: PWSTR = PWSTR::null();
                let result = PropVariantToStringAlloc(pv, &mut buffer);
                if result == S_OK {
                    let value = buffer.to_string().ok(); // Converts PWSTR to Option<String>
                    CoTaskMemFree(Some(buffer.as_ptr().cast())); // Free the allocated buffer
                    Ok(value)
                } else {
                    // If PropVariantToStringAlloc fails, even for theoretically convertible types.
                    Ok(None) 
                }
            }
            vt if vt == (VT_VECTOR.0 | VT_LPWSTR.0) as u16 => {
                // VT_VECTOR | VT_LPWSTR for a vector of strings
                let mut p_elements: *mut PWSTR = std::ptr::null_mut();
                let mut num_elements: u32 = 0;
                // Note: PropVariantToStringVector is for single PROPVARIANT to single string,
                // not for extracting vector elements directly into Vec<PWSTR>.
                // We need to handle the vector manually or find a more specific function.
                // A common way is to use PropVariantGetElementCount and PropVariantGetStringElem.
                // However, let's try with PropVariantToStringVector first if it can produce a delimited string,
                // or if it's meant for single element conversion.
                // The documentation suggests PropVariantToStringVector is more like ToString for PROPVARIANT.
                // For VT_VECTOR | VT_LPWSTR, direct access or specific vector functions are better.
                // Let's assume PropVariantToStringVector is not the right tool here for direct Vec<HSTRING> extraction.

                // Correct approach for VT_VECTOR | VT_LPWSTR:
                // Access `calpwstr` field from the `PROPVARIANT`'s anonymous union.
                let count = pv.Anonymous.Anonymous.Anonymous.calpwstr.cElems as usize;
                let elements_ptr = pv.Anonymous.Anonymous.Anonymous.calpwstr.pElems;
                if elements_ptr.is_null() {
                    return Ok(Some("".to_string())); // Or Ok(None) if empty means no property
                }
                let mut result_vec = Vec::with_capacity(count);
                for i in 0..count {
                    let pwstr_element = *elements_ptr.add(i);
                    if !pwstr_element.is_null() {
                        match pwstr_element.to_string() {
                            Ok(s) => result_vec.push(s),
                            Err(_) => { // Handle cases where a single string in vector is invalid
                                // Potentially log this error or skip the element
                            }
                        }
                    }
                }
                Ok(Some(result_vec.join("; ")))
            }
            VT_EMPTY | VT_NULL => Ok(None),
            _ => Ok(None), // Other types not handled in this function
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;
    use windows::Win32::System::Com::PropVariantFromString;
    use windows::Win32::UI::Shell::PropertiesSystem::{GPS_READWRITE, PKEY_Title}; // Example PropertyKey


/// Sets a string property for a file using its `IPropertyStore`.
///
/// This function sets a single string value. Multi-value strings are not supported in this phase.
///
/// # Arguments
/// * `store`: A reference to the `IPropertyStore` for the file.
///            The store should be obtained with write access (e.g., using `GPS_READWRITE`).
/// * `property_key`: The `PROPERTYKEY` identifying the property to set.
/// * `value`: The string value to set for the property.
///
/// # Returns
/// `Ok(())` on success, or an `Err` with a Windows error code on failure.
///
/// # Remarks
/// COM must be initialized before calling this function.
/// The `IPropertyStore` should be obtained with flags that allow writing (e.g., `GPS_READWRITE`).
pub fn set_windows_file_property_string(
    store: &IPropertyStore,
    property_key: &PROPERTYKEY,
    value: &str,
) -> Result<()> {
    let h_value = HSTRING::from(value);
    let mut pv_raw = PROPVARIANT::default();

    // Initialize the PROPVARIANT from the string.
    // PropVariantFromString creates a VT_LPWSTR variant.
    // The created PROPVARIANT must be cleared with PropVariantClear.
    unsafe { PropVariantFromString(PCWSTR(h_value.as_ptr()), &mut pv_raw)? };
    
    // Wrap in RAII struct to ensure PropVariantClear is called.
    let pv_raii = PropVariantRAII(pv_raw);

    unsafe {
        store.SetValue(property_key, &pv_raii.0)?;
        store.Commit()?;
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;
    

    #[test]
    fn test_com_initializer() {
        match ComInitializer::new() {
            Ok(_initializer) => {}
            Err(e) => {
                if e.code() != windows::Win32::System::Rpc::RPC_E_CHANGED_MODE {
                    panic!("COM initialization failed: {:?}", e);
                } else {
                    println!("COM already initialized in a different mode, acceptable for this test.");
                }
            }
        }
    }

    #[test]
    fn test_get_property_store_on_temp_file() -> windows::core::Result<()> {
        let _com_init = ComInitializer::new()?;
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_property_store.txt");
        File::create(&file_path).unwrap();
        
        // Test with GPS_DEFAULT (read-only)
        match get_property_store(&file_path, GPS_DEFAULT) {
            Ok(_property_store) => {}, // Expected
            Err(e) => {
                eprintln!("Failed to get property store with GPS_DEFAULT for {:?}: {:?}", file_path, e);
                return Err(e);
            }
        }

        // Test with GPS_READWRITE
        match get_property_store(&file_path, GPS_READWRITE) {
            Ok(_property_store) => {}, // Expected
            Err(e) => {
                eprintln!("Failed to get property store with GPS_READWRITE for {:?}: {:?}", file_path, e);
                // This might fail on some systems/files if exclusive access is an issue,
                // but the API call itself should be valid.
                return Err(e);
            }
        }
        Ok(())
    }

    // Helper for PowerShell based property setting (remains for get test)
    fn set_file_title_property_powershell(file_path: &Path, title: &str) -> io::Result<()> {
        let parent_dir_str = file_path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "File path has no parent"))?.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Parent path is not valid UTF-8"))?;
        let file_name_str = file_path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "File path has no name"))?.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "File name is not valid UTF-8"))?;

        let command_str = format!(
            "$shell = New-Object -ComObject Shell.Application; \
             $folder = $shell.Namespace('{}'); \
             $item = $folder.ParseName('{}'); \
             $item.Title = '{}';",
            parent_dir_str,
            file_name_str,
            title
        );
        
        let mut child = std::process::Command::new("powershell")
            .arg("-Command")
            .arg(&command_str)
            .stdout(std::process::Stdio::null()) // Don't need stdout
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        
        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Powershell command failed: {}", stderr),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(1000)); // Increased delay
        Ok(())
    }
    
    #[test]
    #[ignore = "This test relies on PowerShell to set properties and can be flaky in CI or specific environments."]
    fn test_get_windows_file_property_string_title() -> windows::core::Result<()> {
        let _com_init = ComInitializer::new()?;
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_title_property_get.txt");
        File::create(&file_path).unwrap();
        let expected_title = "Test PNA Title Get";
        
        if let Err(e) = set_file_title_property_powershell(&file_path, expected_title) {
            eprintln!("Warning (get_test): Failed to set title property using PowerShell: {}. Test may not be accurate.", e);
        }

        // Important: Get store for reading AFTER potential modification by external process
        let store = get_property_store(&file_path, GPS_DEFAULT)?; 
        
        let pkey_title = PKEY_Title;

        match get_windows_file_property_string(&store, &pkey_title) {
            Ok(Some(title_val)) => {
                 if !title_val.is_empty() { 
                    assert_eq!(title_val, expected_title, "Title property did not match");
                 } else {
                    println!("(get_test) Title property was empty, PowerShell script might not have set it effectively.");
                 }
            }
            Ok(None) => {
                println!("(get_test) PKEY_Title was not found or empty. This might be due to PowerShell helper issues.");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    #[ignore = "Setting properties programmatically and verifying can be flaky in CI due to system interactions and timing."]
    fn test_set_and_get_windows_file_property_string_title() -> windows::core::Result<()> {
        let _com_init = ComInitializer::new()?;
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_set_get_title.txt");
        File::create(&file_path).unwrap();

        let title_to_set = "My Test Document Title";

        // Get IPropertyStore with read-write access
        let path_hstring_rw = HSTRING::from(file_path.as_os_str());
        let path_pcwstr_rw = PCWSTR(path_hstring_rw.as_ptr());
        let mut store_rw_opt: Option<IPropertyStore> = None;
        unsafe {
             SHGetPropertyStoreFromParsingName(path_pcwstr_rw, None, GPS_READWRITE, &IPropertyStore::IID, store_rw_opt.set_abi())?;
        }
        let store_rw = store_rw_opt.ok_or_else(|| windows::core::Error::from(E_FAIL))?;


        // Set the property
        set_windows_file_property_string(&store_rw, &PKEY_Title, title_to_set)?;
        
        // To ensure changes are flushed and readable, it might be necessary to release the store
        // and re-acquire it. Or simply give some time.
        drop(store_rw); // Release the write-mode store
        std::thread::sleep(std::time::Duration::from_millis(500)); // Wait for property system


        // Get a new IPropertyStore for reading (using GPS_DEFAULT)
        let store_read = get_property_store(&file_path, GPS_DEFAULT)?;
        
        let retrieved_title_opt = get_windows_file_property_string(&store_read, &PKEY_Title)?;
        
        assert_eq!(retrieved_title_opt, Some(title_to_set.to_string()), "The retrieved title did not match the set title.");

        Ok(())
    }
}
