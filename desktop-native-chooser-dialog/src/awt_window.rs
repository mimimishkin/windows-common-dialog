use jni::objects::{JClass, JObject};
use jni::sys::{jboolean, jint, jlong, jobject};
use jni::JNIEnv;
use std::ffi::c_void;
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;

#[derive(Debug, Clone)]
pub struct AwtWindowError {
    pub exception_name: &'static str,
    pub message: String,
}

impl Display for AwtWindowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.exception_name, self.message)
    }
}

type WindowRes<T> = Result<T, AwtWindowError>;

#[cfg(target_os = "windows")]
mod win;

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
mod x11;

#[cfg(target_os = "macos")]
mod mac;

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

#[link(name = "jawt", kind = "dylib")]
unsafe extern "system" {
    fn JAWT_GetAWT(env: *mut jni::sys::JNIEnv, awt: *mut JAWT) -> jboolean;
}

fn find_handle(env: &JNIEnv, window: JObject) -> WindowRes<i64> { unsafe {
    let awt = MaybeUninit::<JAWT>::uninit();
    let mut awt = awt.assume_init();
    awt.version = 0x00090000;

    if JAWT_GetAWT(env.get_native_interface(), &mut awt) == 0 {
        return Err(AwtWindowError {
            exception_name: "java/lang/IllegalStateException",
            message: "Failed to get JAWT".into(),
        });
    }

    let ds = (awt.GetDrawingSurface)(env.get_native_interface(), window.into_raw());
    if ds.is_null() {
        return Err(AwtWindowError {
            exception_name: "java/awt/IllegalComponentStateException",
            message: "Failed to GetDrawingSurface".into(),
        });
    }

    let lock_result = ((*ds).Lock)(ds);
    if lock_result & 0x00000001 /* JAWT_LOCK_ERROR */ != 0 {
        (awt.FreeDrawingSurface)(ds);
        return Err(AwtWindowError {
            exception_name: "java/awt/IllegalComponentStateException",
            message: "Failed to lock drawing surface".into(),
        });
    }

    let dsi = ((*ds).GetDrawingSurfaceInfo)(ds);

    Ok((*dsi).window_handle())
} }

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_getWindowNativeHandle0<'a>(
    mut env: JNIEnv<'a>,
    _: JClass<'a>,
    input: JObject<'a>
) -> jlong {
    match find_handle(&env, input) {
        Ok(handle) => handle,
        Err(err) => {
            let _ = env.throw_new(err.exception_name, err.message);
            0
        }
    }
}