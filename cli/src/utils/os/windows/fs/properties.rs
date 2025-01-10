use std::io::Write;
use std::{
    fs, io,
    ops::Deref,
    path::{Path, PathBuf},
};
use windows::{
    core::HSTRING,
    Win32::{
        System::Com::{
            CoTaskMemFree,
            StructuredStorage::{InitPropVariantFromStringAsVector, PropVariantClear},
        },
        UI::Shell::PropertiesSystem::{
            IPropertyStore, PSCoerceToCanonicalValue, PSFormatForDisplayAlloc,
            PSGetNameFromPropertyKey, PSGetPropertyKeyFromName, SHGetPropertyStoreFromParsingName,
            GETPROPERTYSTOREFLAGS, GPS_DEFAULT, PDFF_DEFAULT,
        },
    },
};

pub(crate) fn get_property_store<P: AsRef<Path>>(
    path: P,
    flags: GETPROPERTYSTOREFLAGS,
) -> io::Result<IPropertyStore> {
    let full_path = fs::canonicalize(path)?;
    let path = if let Some(path) = full_path.to_string_lossy().strip_prefix("\\\\?\\") {
        PathBuf::from(path)
    } else {
        full_path
    };
    unsafe { SHGetPropertyStoreFromParsingName(&HSTRING::from(path.deref()), None, flags) }
        .map_err(|e| e.into())
}

pub(crate) fn get_properties<P: AsRef<Path>>(path: P) -> io::Result<Vec<(String, String)>> {
    let store = get_property_store(path, GPS_DEFAULT)?;
    let count = unsafe { store.GetCount() }?;

    let mut properties = Vec::with_capacity(count as usize);
    for i in 0..count {
        let mut key = Default::default();
        unsafe { store.GetAt(i, &mut key) }?;
        let name = if let Ok(name) = unsafe { PSGetNameFromPropertyKey(&key) } {
            unsafe { name.to_hstring() }.to_string_lossy()
        } else {
            format!("{{{:?}}}", key.fmtid)
        };
        let mut value = unsafe { store.GetValue(&key) }?;
        let v = unsafe { PSFormatForDisplayAlloc(&key, &value, PDFF_DEFAULT) }?;
        let s = unsafe { v.to_hstring() }.to_string_lossy();
        unsafe { CoTaskMemFree(Some(v.as_ptr() as _)) };
        unsafe { PropVariantClear(&mut value) }?;
        let kv = (name, s);
        properties.push(kv);
    }
    Ok(properties)
}

pub(crate) fn set_properties<P: AsRef<Path>>(
    path: P,
    properties: impl IntoIterator<Item = (String, String)>,
) -> io::Result<()> {
    let store = get_property_store(path, GPS_DEFAULT)?;
    for (key_name, value) in properties {
        let mut key = Default::default();
        unsafe { PSGetPropertyKeyFromName(&HSTRING::from(key_name), &mut key) }?;

        let mut prop_variant = unsafe { InitPropVariantFromStringAsVector(&HSTRING::from(value)) }?;
        unsafe { PSCoerceToCanonicalValue(&key, &mut prop_variant) }?;
        unsafe { store.SetValue(&key, &prop_variant) }?;
        unsafe { PropVariantClear(&mut prop_variant) }?;
    }
    unsafe { store.Commit() }?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };

    #[test]
    fn get_set_props() {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }.unwrap();
        fs::write("empty.txt", "").unwrap();
        let props = get_properties("empty.txt").unwrap();
        set_properties("empty.txt", props).unwrap();
        fs::remove_file("empty.txt").unwrap();
        unsafe {
            CoUninitialize();
        }
    }
}
