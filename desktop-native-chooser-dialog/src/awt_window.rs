use jni::objects::JObject;
use jni::sys::{jboolean, jint, jobject};
use jni::JNIEnv;
use std::ffi::c_void;
use std::mem::MaybeUninit;

#[cfg(target_os="windows")]
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_Win32DrawingSurfaceInfo {
    hwnd: *mut c_void,
    hdc: *mut c_void,
    lock: *mut c_void,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_X11DrawingSurfaceInfo {
    drawable: c_ulong,
    display: *mut c_void,
    visualID: c_ulong,
    colormapID: c_ulong,
    depth: c_int,
}

#[repr(C)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_Rectangle {
    x: jint,
    y: jint,
    width: jint,
    height: jint,
}

#[repr(C)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_DrawingSurfaceInfo {
    platformInfo: *mut c_void,
    ds: *mut c_void,
    bounds: JAWT_Rectangle,
    clipSize: jint,
    clip: *mut c_void,
}

#[repr(C)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_DrawingSurface {
    env: *mut c_void,
    target: jobject,
    Lock: unsafe extern "system" fn(*mut JAWT_DrawingSurface) -> jint,
    GetDrawingSurfaceInfo: unsafe extern "system" fn(*mut JAWT_DrawingSurface) -> *mut JAWT_DrawingSurfaceInfo,
    FreeDrawingSurfaceInfo: *mut c_void,
    Unlock: *mut c_void,
}

#[repr(C)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT {
    version: jint,
    GetDrawingSurface: unsafe extern "system" fn(*mut jni::sys::JNIEnv, jobject) -> *mut JAWT_DrawingSurface,
    FreeDrawingSurface: unsafe extern "system" fn(*mut JAWT_DrawingSurface),
    Lock: *mut c_void,
    Unlock: *mut c_void,
    GetComponent: *mut c_void,
    CreateEmbeddedFrame: *mut c_void,
    SetBounds: *mut c_void,
    SynthesizeWindowActivation: *mut c_void,
}

#[link(name = "jawt", kind = "raw-dylib")]
unsafe extern "system" {
    fn JAWT_GetAWT(env: *mut jni::sys::JNIEnv, awt: *mut JAWT) -> jboolean;
}

pub(crate) fn find_handle(env: &JNIEnv, window: JObject) -> (Option<&'static str>, i64) { unsafe {
    let awt = MaybeUninit::<JAWT>::uninit();
    let mut awt = awt.assume_init();
    awt.version = 0x00090000;

    if JAWT_GetAWT(env.get_native_interface(), &mut awt) == 0 {
        return (Some("Failed to get JAWT"), 0);
    }

    let ds = (awt.GetDrawingSurface)(env.get_native_interface(), window.into_raw());
    if ds.is_null() {
        return (Some("Failed to GetDrawingSurface"), 0);
    }

    let lock_result = ((*ds).Lock)(ds);
    if lock_result & 0x00000001 /* JAWT_LOCK_ERROR */ != 0 {
        (awt.FreeDrawingSurface)(ds);
        return (Some("Error locking surface"), 0);
    }

    let dsi = ((*ds).GetDrawingSurfaceInfo)(ds);

    #[cfg(target_os = "windows")]
    {
        let dsi_win = (*dsi).platformInfo as *const JAWT_Win32DrawingSurfaceInfo;
        let hwnd = (*dsi_win).hwnd as i64;
        (None, hwnd)
    }

    #[cfg(target_os = "linux")]
    {
        let dsi_x11 = (*dsi).platformInfo as *const JAWT_X11DrawingSurfaceInfo;
        let window = (*dsi_x11).drawable as i64;
        (None, window)
    }

    #[cfg(target_os = "macos")]
    {
        let ptr = (*dsi).platformInfo as i64;
        (None, ptr)
    }
} }