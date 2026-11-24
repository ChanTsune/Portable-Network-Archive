//! Native Windows security descriptor bridge for draft aCLS.
//!
//! A handle is used rather than a pathname API so symlink ACLs can be read
//! without following the link. SACL coverage is only claimed if every known SACL category
//! can be requested in one successful query.
use crate::utils::os::windows::fs::{open_read_acl, open_write_acl};
use pna::{AclChunk, RawChunk};
use std::{ffi::c_void, io, path::Path};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Security::Authorization::{SE_FILE_OBJECT, SetSecurityInfo};
use windows::Win32::Security::{ACL as Win32Acl, OBJECT_SECURITY_INFORMATION};

const DACL: u32 = 0x0000_0004;
const SACL: u32 = 0x0000_0008;
const LABEL: u32 = 0x0000_0010;
const ATTRIBUTE: u32 = 0x0000_0020;
const SCOPE: u32 = 0x0000_0040;
const PROCESS_TRUST_LABEL: u32 = 0x0000_0080;
const ACCESS_FILTER: u32 = 0x0000_0100;
const COMPLETE_SACL: u32 = SACL | LABEL | ATTRIBUTE | SCOPE | PROCESS_TRUST_LABEL | ACCESS_FILTER;
const PROTECTED_DACL: u32 = 0x8000_0000;
const PROTECTED_SACL: u32 = 0x4000_0000;
const SE_DACL_PRESENT: u16 = 0x0004;
const SE_SACL_PRESENT: u16 = 0x0010;
const SE_DACL_PROTECTED: u16 = 0x1000;
const SE_SACL_PROTECTED: u16 = 0x2000;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn SetKernelObjectSecurity(handle: *mut c_void, info: u32, descriptor: *const c_void) -> i32;
    fn GetKernelObjectSecurity(
        handle: *mut c_void,
        info: u32,
        descriptor: *mut c_void,
        length: u32,
        needed: *mut u32,
    ) -> i32;
}

fn query(handle: *mut c_void, info: u32) -> io::Result<Vec<u8>> {
    let mut needed = 0u32;
    // SAFETY: the first call determines the required security descriptor size.
    unsafe { GetKernelObjectSecurity(handle, info, std::ptr::null_mut(), 0, &mut needed) };
    if needed < 20 {
        return Err(io::Error::last_os_error());
    }
    let mut bytes = vec![0u8; needed as usize];
    // SAFETY: valid handle; buffer is writable and the API supplies its size.
    let success = unsafe {
        GetKernelObjectSecurity(handle, info, bytes.as_mut_ptr().cast(), needed, &mut needed)
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    bytes.truncate(needed as usize);
    if bytes.len() < 20 || bytes[4..12] != [0u8; 8] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unexpected owner or group in ACL-only security descriptor",
        ));
    }
    Ok(bytes)
}

/// Captures native DACL, and a complete SACL when the security privilege allows.
///
/// This never mislabels a partial LABEL-only response as complete SACL.
pub(crate) fn capture(path: &Path, follow_links: bool) -> io::Result<Option<RawChunk>> {
    let complete = open_read_acl(path, follow_links, true)
        .and_then(|handle| query(handle.raw().0, DACL | COMPLETE_SACL));
    let (coverage, sd) = match complete {
        Ok(sd) => (3u8, sd),
        Err(_) => {
            let handle = open_read_acl(path, follow_links, false)?;
            (1u8, query(handle.raw().0, DACL)?)
        }
    };
    let mut payload = Vec::with_capacity(sd.len() + 1);
    payload.push(coverage);
    payload.extend_from_slice(&sd);
    let chunk = AclChunk::new(pna::acl::WINDOWS_NT, payload)?;
    chunk.validate()?;
    Ok(Some(chunk.into_raw_chunk()))
}

/// Restores only security-information components declared by the coverage byte.
pub(crate) fn restore(path: &Path, chunk: &AclChunk, follow_links: bool) -> io::Result<()> {
    if chunk.format_id() != pna::acl::WINDOWS_NT {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "not Windows ACL data",
        ));
    }
    chunk.validate()?;
    let payload = chunk.payload();
    let coverage = payload[0];
    let sd = &payload[1..];

    let control = u16::from_le_bytes([sd[2], sd[3]]);
    // SetSecurityInfo cannot represent a covered, absent ACL. Do not
    // reinterpret absence as a NULL ACL (these have different semantics).
    if coverage & 1 != 0 && control & SE_DACL_PRESENT == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "covered but absent Windows DACL cannot be restored",
        ));
    }
    if coverage & 2 != 0 && control & SE_SACL_PRESENT == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "covered but absent Windows SACL cannot be restored",
        ));
    }
    let mut info = 0u32;
    if coverage & 1 != 0 {
        info |= DACL;
        // An unprotected DACL on a newly created file is already inheritable.
        // Requesting UNPROTECTED_DACL here also forces inheritance propagation,
        // which can fail with ACCESS_DENIED even when WRITE_DAC is available.
        // Apply protection explicitly only when the archived DACL requires it.
        if control & SE_DACL_PROTECTED != 0 {
            info |= PROTECTED_DACL;
        }
    }
    if coverage & 2 != 0 {
        info |= COMPLETE_SACL;
        if control & SE_SACL_PROTECTED != 0 {
            info |= PROTECTED_SACL;
        }
    }
    // The wire descriptor is self-relative. Its offsets refer to its first
    // byte. Native ACL parameters must have suitable memory alignment.
    let mut aligned = vec![0u32; sd.len().div_ceil(4)];
    // SAFETY: the destination is an allocated u32 buffer large enough for sd.
    let aligned_bytes = unsafe {
        std::slice::from_raw_parts_mut(
            aligned.as_mut_ptr().cast::<u8>(),
            aligned.len() * std::mem::size_of::<u32>(),
        )
    };
    aligned_bytes[..sd.len()].copy_from_slice(sd);
    let base = aligned.as_ptr().cast::<u8>();
    let acl_ptr = |offset: usize| -> io::Result<Option<*const Win32Acl>> {
        if offset == 0 {
            return Ok(None);
        }
        if offset < 20 || offset >= sd.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Windows ACL offset lies outside the descriptor",
            ));
        }
        // SAFETY: the offset is inside the aligned buffer.
        Ok(Some(unsafe { base.add(offset).cast::<Win32Acl>() }))
    };
    let dacl_offset = u32::from_le_bytes(sd[16..20].try_into().unwrap()) as usize;
    let sacl_offset = u32::from_le_bytes(sd[12..16].try_into().unwrap()) as usize;
    let dacl = if coverage & 1 != 0 {
        acl_ptr(dacl_offset)?
    } else {
        None
    };
    let sacl = if coverage & 2 != 0 {
        acl_ptr(sacl_offset)?
    } else {
        None
    };
    let handle = open_write_acl(path, follow_links, coverage & 2 != 0)?;
    // SAFETY: SetSecurityInfo reads native ACL pointers from the aligned
    // descriptor while the backing buffer remains live.
    let status = unsafe {
        SetSecurityInfo(
            handle.raw(),
            SE_FILE_OBJECT,
            OBJECT_SECURITY_INFORMATION(info),
            None,
            None,
            dacl,
            sacl,
        )
    };
    if status != ERROR_SUCCESS {
        // Probe the native descriptor setter when the ACL setter denies access.
        // Both operate on the same handle and require WRITE_DAC for DACL-only data.
        if status.0 == 5 && coverage == 1 {
            // SAFETY: the validated self-relative descriptor is aligned and live.
            let fallback = unsafe {
                SetKernelObjectSecurity(handle.raw().0 as *mut c_void, DACL, base.cast())
            };
            if fallback != 0 {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "SetSecurityInfo returned ACCESS_DENIED; SetKernelObjectSecurity also failed: {}",
                    io::Error::last_os_error()
                ),
            ));
        }
        return Err(io::Error::new(
            io::Error::from_raw_os_error(status.0 as i32).kind(),
            format!(
                "SetSecurityInfo failed (status={}, coverage={coverage:#04x}, info={info:#010x}, control={control:#06x}, dacl_offset={dacl_offset}, sacl_offset={sacl_offset}): {}",
                status.0,
                io::Error::from_raw_os_error(status.0 as i32)
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_ownerless_relative_descriptor_validity() -> io::Result<()> {
        // Validate the actual ACL-only descriptor returned by Windows rather
        // than inferring validity from our structural aCLS decoder.
        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn IsValidSecurityDescriptor(descriptor: *const c_void) -> i32;
        }
        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlValidRelativeSecurityDescriptor(
                descriptor: *const c_void,
                length: u32,
                information: u32,
            ) -> u8;
        }

        let folder =
            std::env::temp_dir().join(format!("pna-acl-ownerless-validity-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let path = folder.join("original.tmp");
        let result = (|| {
            std::fs::write(&path, b"ACL")?;
            let handle = open_read_acl(&path, true, false)?;
            let sd = query(handle.raw().0, DACL)?;
            let control = u16::from_le_bytes([sd[2], sd[3]]);
            assert_ne!(control & 0x8000, 0, "descriptor must be self-relative");
            assert_eq!(&sd[4..12], &[0; 8], "owner and group must be absent");

            // A self-relative descriptor may have different validity rules
            // between the Win32 and NT APIs. Record both results independently;
            // do not switch wire representations based on one API alone.
            let win32_valid = unsafe { IsValidSecurityDescriptor(sd.as_ptr().cast()) } != 0;
            let nt_valid = unsafe {
                RtlValidRelativeSecurityDescriptor(sd.as_ptr().cast(), sd.len() as u32, DACL)
            } != 0;
            assert!(
                win32_valid && nt_valid,
                "ownerless self-relative SD: IsValidSecurityDescriptor={win32_valid}, RtlValidRelativeSecurityDescriptor={nt_valid}, control={control:#06x}"
            );
            Ok(())
        })();
        std::fs::remove_dir_all(folder)?;
        result
    }

    #[test]
    fn native_ownerless_descriptor_acl_state_matrix() {
        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn IsValidSecurityDescriptor(descriptor: *const c_void) -> i32;
        }
        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlValidRelativeSecurityDescriptor(
                descriptor: *const c_void,
                length: u32,
                information: u32,
            ) -> u8;
        }

        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn GetSecurityDescriptorDacl(
                descriptor: *const c_void,
                present: *mut i32,
                acl: *mut *mut c_void,
                defaulted: *mut i32,
            ) -> i32;
            fn GetSecurityDescriptorSacl(
                descriptor: *const c_void,
                present: *mut i32,
                acl: *mut *mut c_void,
                defaulted: *mut i32,
            ) -> i32;
            fn IsValidAcl(acl: *const c_void) -> i32;
        }

        // The first candidate is an ordinary native self-relative descriptor.
        // Exercise all three states independently for DACL and SACL: absent,
        // present but NULL, and present with a structurally empty ACL.
        // These are API-validity probes, not proof of filesystem restoration.
        for dacl_state in 0..3u8 {
            for sacl_state in 0..3u8 {
                let mut sd = vec![0u8; 20];
                sd[0] = 1;
                let mut control = 0x8000u16;
                let mut requested = 0u32;
                for (state, offset_start, present, info) in [
                    (dacl_state, 16usize, SE_DACL_PRESENT, DACL),
                    (sacl_state, 12usize, SE_SACL_PRESENT, SACL),
                ] {
                    if state != 0 {
                        control |= present;
                        requested |= info;
                    }
                    if state == 2 {
                        let offset = u32::try_from(sd.len()).unwrap();
                        sd[offset_start..offset_start + 4].copy_from_slice(&offset.to_le_bytes());
                        // ACL_REVISION=2; AclSize=8; AceCount=0.
                        sd.extend_from_slice(&[2, 0, 8, 0, 0, 0, 0, 0]);
                    }
                }
                sd[2..4].copy_from_slice(&control.to_le_bytes());

                let win32_valid = unsafe { IsValidSecurityDescriptor(sd.as_ptr().cast()) } != 0;
                let nt_valid = unsafe {
                    RtlValidRelativeSecurityDescriptor(
                        sd.as_ptr().cast(),
                        sd.len() as u32,
                        requested,
                    )
                } != 0;
                assert!(
                    win32_valid && nt_valid,
                    "ownerless DACL state={dacl_state}, SACL state={sacl_state}: \
                     IsValidSecurityDescriptor={win32_valid}, \
                     RtlValidRelativeSecurityDescriptor={nt_valid}, \
                     control={control:#06x}, requested={requested:#010x}"
                );
                // Ask the native Windows parser whether each ACL is absent,
                // NULL, or an actual zero-ACE ACL. Structural validity alone
                // cannot prove these three states remain distinguishable.
                for (state, getter, name) in [
                    (
                        dacl_state,
                        GetSecurityDescriptorDacl
                            as unsafe extern "system" fn(
                                *const c_void,
                                *mut i32,
                                *mut *mut c_void,
                                *mut i32,
                            ) -> i32,
                        "DACL",
                    ),
                    (
                        sacl_state,
                        GetSecurityDescriptorSacl
                            as unsafe extern "system" fn(
                                *const c_void,
                                *mut i32,
                                *mut *mut c_void,
                                *mut i32,
                            ) -> i32,
                        "SACL",
                    ),
                ] {
                    let mut present = -1i32;
                    let mut acl = std::ptr::null_mut::<c_void>();
                    let mut defaulted = -1i32;
                    // SAFETY: sd is a complete native self-relative descriptor;
                    // the output pointers refer to live stack variables.
                    let ok = unsafe {
                        getter(sd.as_ptr().cast(), &mut present, &mut acl, &mut defaulted)
                    };
                    assert_ne!(ok, 0, "{name} native descriptor inspection failed");
                    assert_eq!(present != 0, state != 0, "{name} presence changed");
                    assert_eq!(!acl.is_null(), state == 2, "{name} NULL/empty changed");
                    if state == 2 {
                        // ACL header: revision (1), sbz1 (1), size (2),
                        // AceCount (2), sbz2 (2).
                        // SAFETY: Windows returned a pointer to an ACL within sd.
                        let ace_count = unsafe {
                            u16::from_le_bytes([
                                *acl.cast::<u8>().add(4),
                                *acl.cast::<u8>().add(5),
                            ])
                        };
                        assert_eq!(ace_count, 0, "{name} empty ACL acquired ACEs");
                        // Validate the native ACL itself, not only the enclosing SD.
                        // SAFETY: the ACL pointer comes from a validated live SD.
                        assert_ne!(
                            unsafe { IsValidAcl(acl.cast_const()) },
                            0,
                            "{name} empty ACL is not accepted by Windows"
                        );
                    }
                    assert_eq!(defaulted, 0, "{name} ACL defaulted flag changed");
                }
            }
        }
    }

    #[test]
    fn native_empty_protected_dacl_pna_roundtrip_on_distinct_file() -> io::Result<()> {
        use pna::{Archive, Chunk, FileEntryBuilder};

        // An empty DACL denies access; unlike a NULL DACL it is not a
        // permissive ACL. Keep the source file untouched and restore to a
        // separate file whose handle is opened before changing its ACL.
        let folder = std::env::temp_dir().join(format!(
            "pna-acl-empty-dacl-{}", std::process::id()
        ));
        std::fs::create_dir_all(&folder)?;
        let restored = folder.join("restored.tmp");
        std::fs::write(&restored, b"ACL")?;
        let result = (|| {
            let mut sd = vec![0u8; 20];
            sd[0] = 1;
            let control = 0x8000u16 | SE_DACL_PRESENT | SE_DACL_PROTECTED;
            sd[2..4].copy_from_slice(&control.to_le_bytes());
            sd[16..20].copy_from_slice(&20u32.to_le_bytes());
            // ACL_REVISION=2, AclSize=8, AceCount=0.
            sd.extend_from_slice(&[2, 0, 8, 0, 0, 0, 0, 0]);
            let mut payload = vec![1u8]; // DACL-only coverage
            payload.extend_from_slice(&sd);
            let original = AclChunk::new(pna::acl::WINDOWS_NT, payload)?;
            original.validate()?;

            let mut bytes = Vec::new();
            {
                let mut archive = Archive::write_header(&mut bytes)?;
                let mut builder = FileEntryBuilder::new("empty-dacl.txt".into())?;
                builder.add_extra_chunk(original.clone().into_raw_chunk());
                archive.add_entry(builder.build()?)?;
                archive.finalize()?;
            }
            let mut archive = Archive::read_header(&bytes[..])?;
            let entry = archive.entries().skip_solid().next().unwrap()?;
            let decoded = entry.acl_chunk()?.expect("empty DACL aCLS in archive");
            assert_eq!(decoded, original, "PNA modified the empty DACL");

            // The file must remain accessible for ACL inspection even after
            // applying the empty DACL. Open the read handle in advance.
            let read_handle = open_read_acl(&restored, true, false)?;
            restore(&restored, &decoded, true)?;
            let observed = query(read_handle.raw().0, DACL)?;
            let observed_control = u16::from_le_bytes([observed[2], observed[3]]);
            assert_ne!(
                observed_control & SE_DACL_PRESENT,
                0,
                "empty DACL became absent"
            );
            assert_ne!(
                observed_control & SE_DACL_PROTECTED,
                0,
                "protected DACL lost its inheritance protection"
            );
            let offset =
                u32::from_le_bytes(observed[16..20].try_into().unwrap()) as usize;
            assert!(offset >= 20 && offset + 8 <= observed.len(), "DACL missing");
            assert_eq!(
                u16::from_le_bytes([observed[offset + 4], observed[offset + 5]]),
                0,
                "empty DACL gained ACEs after native restore"
            );
            Ok(())
        })();
        // Removing a file with a deny-all DACL can require restoring access;
        // the temporary directory is not used for any other test.
        let _ = std::fs::remove_dir_all(&folder);
        result
    }

    #[test]
    fn native_audit_and_label_sacl_archive_roundtrip() -> io::Result<()> {
        use pna::{Archive, Chunk, FileEntryBuilder};

        #[repr(C)]
        struct Luid {
            low: u32,
            high: i32,
        }

        #[repr(C)]
        struct TokenPrivileges {
            count: u32,
            luid: Luid,
            attributes: u32,
        }

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
            fn CloseHandle(handle: *mut c_void) -> i32;
            fn GetLastError() -> u32;
            fn LocalFree(memory: *mut c_void) -> *mut c_void;
        }

        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn OpenProcessToken(
                process: *mut c_void,
                access: u32,
                token: *mut *mut c_void,
            ) -> i32;
            fn LookupPrivilegeValueW(
                system: *const u16,
                name: *const u16,
                luid: *mut Luid,
            ) -> i32;
            fn AdjustTokenPrivileges(
                token: *mut c_void,
                disable_all: i32,
                state: *const TokenPrivileges,
                size: u32,
                previous: *mut TokenPrivileges,
                returned: *mut u32,
            ) -> i32;
            fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text: *const u16,
                revision: u32,
                descriptor: *mut *mut c_void,
                size: *mut u32,
            ) -> i32;
        }

        struct Token(*mut c_void);
        impl Drop for Token {
            fn drop(&mut self) {
                // SAFETY: the token is an owned handle.
                unsafe { CloseHandle(self.0) };
            }
        }
        struct Privilege<'a> {
            token: &'a Token,
            previous: TokenPrivileges,
        }
        impl Drop for Privilege<'_> {
            fn drop(&mut self) {
                // SAFETY: previous is the state returned by AdjustTokenPrivileges.
                unsafe {
                    AdjustTokenPrivileges(
                        self.token.0,
                        0,
                        &self.previous,
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
            }
        }
        struct Descriptor(*mut c_void);
        impl Drop for Descriptor {
            fn drop(&mut self) {
                // SAFETY: the SDDL conversion allocated this buffer with LocalAlloc.
                unsafe { LocalFree(self.0) };
            }
        }

        fn at(stage: &str, error: io::Error) -> io::Error {
            io::Error::new(error.kind(), format!("{stage}: {error}"))
        }

        // Do not accept DACL-only fallback as evidence of a full SACL query.
        // The CI runner must actually hold SeSecurityPrivilege.
        let mut token = std::ptr::null_mut();
        // SAFETY: the current process pseudo-handle and output pointer are valid.
        if unsafe { OpenProcessToken(GetCurrentProcess(), 0x28, &mut token) } == 0 {
            return Err(at("OpenProcessToken", io::Error::last_os_error()));
        }
        let token = Token(token);
        let name: Vec<u16> = "SeSecurityPrivilege\0".encode_utf16().collect();
        let mut state = TokenPrivileges {
            count: 1,
            luid: Luid { low: 0, high: 0 },
            attributes: 2, // SE_PRIVILEGE_ENABLED
        };
        // SAFETY: name is terminated and luid points to writable storage.
        if unsafe {
            LookupPrivilegeValueW(std::ptr::null(), name.as_ptr(), &mut state.luid)
        } == 0
        {
            return Err(at("LookupPrivilegeValueW", io::Error::last_os_error()));
        }
        let mut previous = TokenPrivileges {
            count: 0,
            luid: Luid { low: 0, high: 0 },
            attributes: 0,
        };
        let mut returned = 0;
        // SAFETY: both TOKEN_PRIVILEGES structures have space for one entry.
        let adjusted = unsafe {
            AdjustTokenPrivileges(
                token.0,
                0,
                &state,
                std::mem::size_of::<TokenPrivileges>() as u32,
                &mut previous,
                &mut returned,
            )
        };
        // AdjustTokenPrivileges may return success with ERROR_NOT_ALL_ASSIGNED.
        let error = unsafe { GetLastError() };
        if adjusted == 0 || error != 0 {
            return Err(at(
                "enable SeSecurityPrivilege",
                io::Error::from_raw_os_error(error as i32),
            ));
        }
        let _privilege = Privilege {
            token: &token,
            previous,
        };

        // Parse the actual native ACE sequence, excluding unused ACL tail space.
        fn sacl_aces(sd: &[u8]) -> io::Result<Vec<Vec<u8>>> {
            let offset = u32::from_le_bytes(sd[12..16].try_into().unwrap()) as usize;
            if offset == 0 {
                return Err(io::Error::other("expected a non-NULL SACL"));
            }
            let acl = sd
                .get(offset..)
                .filter(|acl| acl.len() >= 8)
                .ok_or_else(|| io::Error::other("truncated SACL"))?;
            let length = u16::from_le_bytes([acl[2], acl[3]]) as usize;
            let count = u16::from_le_bytes([acl[4], acl[5]]);
            let acl = acl
                .get(..length)
                .filter(|acl| acl.len() >= 8)
                .ok_or_else(|| io::Error::other("invalid SACL size"))?;
            let mut cursor = 8;
            let mut aces = Vec::new();
            for _ in 0..count {
                let header = acl
                    .get(cursor..cursor + 4)
                    .ok_or_else(|| io::Error::other("truncated SACL ACE header"))?;
                let size = u16::from_le_bytes([header[2], header[3]]) as usize;
                let ace = acl
                    .get(cursor..cursor + size)
                    .filter(|ace| ace.len() >= 4)
                    .ok_or_else(|| io::Error::other("invalid SACL ACE size"))?;
                aces.push(ace.to_vec());
                cursor += size;
            }
            Ok(aces)
        }

        let folder =
            std::env::temp_dir().join(format!("pna-acl-sacl-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let source = folder.join("source.tmp");
        let destination = folder.join("destination.tmp");
        let result = (|| {
            std::fs::write(&source, b"ACL")?;
            std::fs::write(&destination, b"ACL")?;

            // Use the native SDDL parser to create independent audit and
            // integrity-label ACEs, not bytes generated by the PNA codec.
            let sddl: Vec<u16> = "S:(AU;SA;FR;;;WD)(ML;;NW;;;ME)\0"
                .encode_utf16()
                .collect();
            let mut descriptor = std::ptr::null_mut();
            // SAFETY: the terminated SDDL and output pointer are valid.
            if unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    1,
                    &mut descriptor,
                    std::ptr::null_mut(),
                )
            } == 0
            {
                return Err(at("parse SACL fixture", io::Error::last_os_error()));
            }
            let descriptor = Descriptor(descriptor);
            let handle = open_write_acl(&source, true, true)
                .map_err(|e| at("open source for SACL write", e))?;
            // SAFETY: the descriptor is a live native self-relative descriptor.
            if unsafe {
                SetKernelObjectSecurity(handle.raw().0 as *mut c_void, SACL | LABEL, descriptor.0.cast_const())
            } == 0
            {
                return Err(at("set native audit and label", io::Error::last_os_error()));
            }
            let handle = open_read_acl(&source, true, true)
                .map_err(|e| at("open source for SACL read", e))?;
            let audit = sacl_aces(&query(handle.raw().0, SACL).map_err(|e| at("query audit", e))?)?;
            let label = sacl_aces(&query(handle.raw().0, LABEL).map_err(|e| at("query label", e))?)?;
            let expected = sacl_aces(
                &query(handle.raw().0, DACL | COMPLETE_SACL)
                    .map_err(|e| at("query all SACL categories", e))?,
            )?;
            assert!(audit.iter().any(|ace| ace[0] == 0x02), "missing audit ACE");
            assert!(label.iter().any(|ace| ace[0] == 0x11), "missing label ACE");
            for ace in audit.iter().chain(&label) {
                assert!(expected.contains(ace), "combined query lost an ACE");
            }

            let captured = capture(&source, true)?.expect("captured source ACL");
            let original = AclChunk::from_bytes(captured.data())?;
            assert_eq!(original.payload()[0], 3, "full SACL capture fell back to DACL");
            assert_eq!(sacl_aces(&original.payload()[1..])?, expected);

            let mut bytes = Vec::new();
            {
                let mut archive = Archive::write_header(&mut bytes)?;
                let mut builder = FileEntryBuilder::new("sacl.txt".into())?;
                builder.add_extra_chunk(captured);
                archive.add_entry(builder.build()?)?;
                archive.finalize()?;
            }
            let mut archive = Archive::read_header(&bytes[..])?;
            let entry = archive.entries().skip_solid().next().unwrap()?;
            let decoded = entry.acl_chunk()?.expect("archived DACL and SACL");
            assert_eq!(decoded, original);
            if let Err(error) = restore(&destination, &decoded, true) {
                // The fixture succeeded with SetKernelObjectSecurity(SACL |
                // LABEL), while restore requests every SACL category via
                // SetSecurityInfo. Isolate the setter and information-mask
                // variables on fresh objects rather than dropping categories.
                let sd = &decoded.payload()[1..];
                let mut aligned = vec![0u32; sd.len().div_ceil(4)];
                // SAFETY: the allocation contains at least sd.len() bytes.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        sd.as_ptr(),
                        aligned.as_mut_ptr().cast::<u8>(),
                        sd.len(),
                    );
                }
                let base = aligned.as_ptr().cast::<u8>();
                let dacl_offset = u32::from_le_bytes(sd[16..20].try_into().unwrap()) as usize;
                let sacl_offset = u32::from_le_bytes(sd[12..16].try_into().unwrap()) as usize;
                assert!(dacl_offset >= 20 && sacl_offset >= 20);
                // SAFETY: decoded is a validated descriptor with non-NULL ACLs.
                let dacl = Some(unsafe { base.add(dacl_offset).cast::<Win32Acl>() });
                let sacl = Some(unsafe { base.add(sacl_offset).cast::<Win32Acl>() });
                for (index, mask) in [
                    DACL | COMPLETE_SACL,
                    DACL | SACL | LABEL,
                    SACL | LABEL,
                    DACL | SACL | LABEL | ATTRIBUTE,
                    DACL | SACL | LABEL | SCOPE,
                    DACL | SACL | LABEL | PROCESS_TRUST_LABEL,
                    DACL | SACL | LABEL | ACCESS_FILTER,
                ]
                .into_iter()
                .enumerate()
                {
                    for kernel_setter in [false, true] {
                        let probe = folder.join(format!("probe-{index}-{kernel_setter}.tmp"));
                        std::fs::write(&probe, b"ACL")?;
                        let handle = open_write_acl(&probe, true, true)?;
                        // SAFETY: the descriptor and its ACL pointers remain live.
                        let status = unsafe {
                            if kernel_setter {
                                if SetKernelObjectSecurity(handle.raw().0, mask, base.cast()) == 0 {
                                    GetLastError()
                                } else {
                                    0
                                }
                            } else {
                                SetSecurityInfo(
                                    handle.raw(),
                                    SE_FILE_OBJECT,
                                    OBJECT_SECURITY_INFORMATION(mask),
                                    None,
                                    None,
                                    if mask & DACL != 0 { dacl } else { None },
                                    sacl,
                                )
                                .0
                            }
                        };
                        let observed = if status == 0 {
                            open_read_acl(&probe, true, true)
                                .and_then(|h| query(h.raw().0, DACL | COMPLETE_SACL))
                                .and_then(|sd| sacl_aces(&sd))
                                .map(|aces| aces == expected)
                        } else {
                            Ok(false)
                        };
                        eprintln!(
                            "SACL setter probe: kernel={kernel_setter}, mask={mask:#010x}, status={status}, same_sacl={observed:?}"
                        );
                    }
                }
                return Err(at("restore full ACL", error));
            }
            let after = capture(&destination, true)?.expect("restored destination ACL");
            let after = AclChunk::from_bytes(after.data())?;
            assert_eq!(after.payload()[0], 3, "restored SACL coverage missing");
            assert_eq!(sacl_aces(&after.payload()[1..])?, expected, "native SACL changed");
            Ok(())
        })();
        std::fs::remove_dir_all(folder)?;
        result
    }

    #[test]
    fn native_windows_file_dacl_roundtrip() -> io::Result<()> {
        let folder =
            std::env::temp_dir().join(format!("pna-acl-draft-windows-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let original_file = folder.join("original.tmp");
        let restored_file = folder.join("restored.tmp");
        std::fs::write(&original_file, b"ACL")?;
        std::fs::write(&restored_file, b"ACL")?;
        let result = (|| {
            let encoded = capture(&original_file, true)
                .map_err(|e| {
                    io::Error::new(e.kind(), format!("capture original Windows ACL: {e}"))
                })?
                .expect("Windows file DACL");
            use pna::{Archive, Chunk, FileEntryBuilder};
            let original = AclChunk::from_bytes(encoded.data())?;
            original.validate()?;
            let mut bytes = Vec::new();
            {
                let mut archive = Archive::write_header(&mut bytes)?;
                let mut builder = FileEntryBuilder::new("windows.txt".into())?;
                builder.add_extra_chunk(encoded);
                archive.add_entry(builder.build()?)?;
                archive.finalize()?;
            }
            let mut archive = Archive::read_header(&bytes[..])?;
            let entry = archive.entries().skip_solid().next().unwrap()?;
            let chunk = entry.acl_chunk()?.expect("aCLS in PNA file");
            assert_eq!(chunk.format_id(), pna::acl::WINDOWS_NT);
            chunk.validate()?;
            assert_eq!(chunk, original, "PNA must preserve the native ACL payload");
            restore(&restored_file, &chunk, true)
                .map_err(|e| io::Error::new(e.kind(), format!("restore Windows ACL: {e}")))?;
            let after = capture(&restored_file, true)
                .map_err(|e| {
                    io::Error::new(e.kind(), format!("capture restored Windows ACL: {e}"))
                })?
                .expect("restored Windows DACL");
            let after = AclChunk::from_bytes(after.data())?;
            assert_eq!(after.format_id(), pna::acl::WINDOWS_NT);
            after.validate()?;
            assert_eq!(
                after, original,
                "native DACL changed after PNA roundtrip onto a different file"
            );
            Ok(())
        })();
        std::fs::remove_dir_all(folder)?;
        result
    }
}
