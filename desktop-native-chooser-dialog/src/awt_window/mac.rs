use crate::awt_window::JAWT_DrawingSurfaceInfo;
use crate::mac::with_main_thread;
use objc2::__framework_prelude::Retained;
use objc2_app_kit::{NSApplication, NSWindow};
use objc2_foundation::{NSArray, NSString};

impl JAWT_DrawingSurfaceInfo {
    pub fn window_handle(&self) -> i64 {
        let window = self.platformInfo.cast::<NSWindow>() as isize;
        window as i64
    }
}