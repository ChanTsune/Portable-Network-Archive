use libc::memcpy;
use std::io::Write;
use std::{
    fs, io,
    ops::Deref,
    path::{Path, PathBuf},
};
use windows::{
    core::{HSTRING, PWSTR},
    Win32::{
        System::{
            Com::{
                CoTaskMemAlloc, CoTaskMemFree,
                StructuredStorage::{
                    PropVariantClear, PropVariantToStringAlloc, PROPVARIANT, PROPVARIANT_0_0,
                    PROPVARIANT_0_0_0,
                },
            },
            Variant::VT_LPWSTR,
        },
        UI::Shell::PropertiesSystem::{
            IPropertyStore, PSCoerceToCanonicalValue, PSGetNameFromPropertyKey,
            PSGetPropertyKeyFromName, SHGetPropertyStoreFromParsingName, GETPROPERTYSTOREFLAGS,
            GPS_DEFAULT,
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
        // let v = unsafe { PSFormatForDisplayAlloc(&key, &value, PDFF_DEFAULT) }?;
        let v = unsafe { PropVariantToStringAlloc(&value) }.unwrap();
        let s = unsafe { v.to_hstring() }.to_string_lossy();
        unsafe { CoTaskMemFree(Some(v.as_ptr() as _)) };
        unsafe { PropVariantClear(&mut value) }?;
        let kv = (name, s);
        properties.push(kv);
    }
    Ok(properties)
}

#[allow(non_snake_case)]
unsafe fn InitPropVariantFromString<P0>(psz: P0) -> windows::core::Result<PROPVARIANT>
where
    P0: windows::core::Param<windows::core::PCWSTR>,
{
    let param = psz.param().abi();
    let byte_count = (param.len() + 1) * std::mem::size_of::<u16>();
    let mem = CoTaskMemAlloc(byte_count);
    memcpy(mem, param.as_ptr() as _, byte_count);
    let param = PWSTR::from_raw(mem as _);
    let anonymous = PROPVARIANT_0_0 {
        vt: VT_LPWSTR,
        wReserved1: 0,
        wReserved2: 0,
        wReserved3: 0,
        Anonymous: PROPVARIANT_0_0_0 { pwszVal: param },
    };
    let mut propvar = PROPVARIANT::default();
    propvar.Anonymous.Anonymous = std::mem::ManuallyDrop::new(anonymous);
    Ok(propvar)
}

pub(crate) fn set_properties<P: AsRef<Path>>(
    path: P,
    properties: impl IntoIterator<Item = (String, String)>,
) -> io::Result<()> {
    let store = get_property_store(path, GPS_DEFAULT)?;
    for (key_name, value) in properties {
        io::stdout()
            .lock()
            .write_all(format!("k: {}, v: {}\n", key_name, value).as_bytes())
            .unwrap();
        let mut key = Default::default();
        if let Ok(_) = unsafe { PSGetPropertyKeyFromName(&HSTRING::from(key_name), &mut key) } {
            let mut prop_variant =
                unsafe { InitPropVariantFromString(&HSTRING::from(value)) }.unwrap();
            unsafe { PSCoerceToCanonicalValue(&key, &mut prop_variant) }.unwrap();
            unsafe { store.SetValue(&key, &prop_variant) }.unwrap();
            unsafe { PropVariantClear(&mut prop_variant) }.unwrap();
        }
    }
    unsafe { store.Commit() }.unwrap();
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
        let _props = get_properties("empty.txt").unwrap();
        // set_properties("empty.txt", props).unwrap();
        fs::remove_file("empty.txt").unwrap();
        unsafe {
            CoUninitialize();
        }
    }
}
