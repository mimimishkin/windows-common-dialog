use crate::awt_window::JAWT_DrawingSurfaceInfo;
use std::ffi::{c_int, c_ulong, c_void};

#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_X11DrawingSurfaceInfo {
    pub drawable: c_ulong,
    pub display: *mut c_void,
    pub visualID: c_ulong,
    pub colormapID: c_ulong,
    pub depth: c_int,
}

impl JAWT_DrawingSurfaceInfo {
    pub fn window_handle(&self) -> i64 {
        let info = self.platformInfo.cast::<JAWT_X11DrawingSurfaceInfo>();
        let window = unsafe { (*info).drawable };
        window as i64
    }
}