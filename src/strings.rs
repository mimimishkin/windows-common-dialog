//! Loading and formatting localized strings from the system libraries.

use windows::Win32::Foundation::HLOCAL;
use windows::Win32::Foundation::LocalFree;
use windows::Win32::System::Diagnostics::Debug::{
    FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_ARGUMENT_ARRAY, FORMAT_MESSAGE_FROM_STRING,
    FormatMessageW,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::LoadStringW;
use windows::core::{Error, HSTRING, PCWSTR, PWSTR};
use crate::utils::WinRes;

/// String resource IDs used by the dialogs.
pub mod id {
    /// `OK` (`user32.dll`).
    pub const OK: u32 = 800;

    /// `Open` (`comdlg32.dll`).
    pub const OPEN: u32 = 384;

    /// `Open File` (`comdlg32.dll`).
    pub const OPEN_FILE: u32 = 436;

    /// `Select Folder` (`comdlg32.dll`).
    pub const SELECT_FOLDER: u32 = 439;

    /// `Path not found` (`comdlg32.dll`).
    pub const PATH_NOT_FOUND: u32 = 392;
}

/// Loads a string resource from a module that is already loaded, e.g. `w!("comdlg32.dll")`.
pub fn load(module: PCWSTR, id: u32) -> WinRes<HSTRING> {
    let module = unsafe { GetModuleHandleW(module) }?;
    let mut buffer = vec![0u16; 128];

    loop {
        let len = unsafe {
            LoadStringW(
                Some(module.into()),
                id,
                PWSTR(buffer.as_mut_ptr()),
                buffer.len() as _,
            )
        };
        if len == 0 {
            return Err(Error::from_thread());
        }
        if (len as usize) < buffer.len() - 1 {
            return Ok(HSTRING::from_wide(&buffer[..len as usize]));
        }
        // The string may have been truncated; retry with a bigger buffer.
        buffer.resize(buffer.len() * 2, 0);
    }
}

/// Formats a `FormatMessage`-style template (e.g. a string loaded with [`load`]).
///
/// `args` supplies one pointer per `%1`, `%2`, ... placeholder.
pub fn format(template: &HSTRING, args: &[*const u16]) -> WinRes<HSTRING> {
    let mut buffer: *mut u16 = std::ptr::null_mut();
    let len = unsafe {
        FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER
                | FORMAT_MESSAGE_FROM_STRING
                | FORMAT_MESSAGE_ARGUMENT_ARRAY,
            Some(template.as_ptr() as *const _),
            0,
            0,
            PWSTR(&mut buffer as *mut *mut u16 as *mut u16),
            0,
            Some(args.as_ptr() as *const _),
        )
    };
    if len == 0 {
        return Err(Error::from_thread());
    }

    let text = HSTRING::from_wide(unsafe { std::slice::from_raw_parts(buffer, len as usize) });
    unsafe { LocalFree(Some(HLOCAL(buffer as *mut _))) };
    Ok(text)
}
