use crate::awt_window::JAWT_DrawingSurfaceInfo;
use std::ffi::c_void;

#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_Win32DrawingSurfaceInfo {
    pub hwnd: *mut c_void,
    pub hdc: *mut c_void,
    pub hpalette: *mut c_void,
}

impl JAWT_DrawingSurfaceInfo {
    pub fn window_handle(&self) -> i64 {
        let info = self.platformInfo.cast::<JAWT_Win32DrawingSurfaceInfo>();
        let hwnd = unsafe { (*info).hwnd };
        hwnd as i64
    }
}