use std::mem;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Returns true if the current process is running with an elevated
/// (Administrator) token.
pub(crate) fn is_running_as_root() -> bool {
    let mut token = HANDLE::default();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.is_err() {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION::default();
    let mut returned_len = 0u32;
    let result = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned_len,
        )
    };
    let _ = unsafe { CloseHandle(token) };
    result.is_ok() && elevation.TokenIsElevated != 0
}
