use crate::java::awt_window::JAWT_DrawingSurfaceInfo;
use std::ffi::c_void;
use std::ops::Deref;
use windows::Win32::Foundation::{FALSE, HWND, LPARAM, RECT, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW};
use windows_core::{BOOL, HSTRING};

#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct JAWT_Win32DrawingSurfaceInfo {
    pub hwnd: *mut c_void,
    pub hdc: *mut c_void,
    pub lock: *mut c_void,
}

impl JAWT_DrawingSurfaceInfo {
    pub fn window_handle(&self) -> i64 {
        let info = self.platformInfo.cast::<JAWT_Win32DrawingSurfaceInfo>();
        let hwnd = unsafe { (*info).hwnd };
        hwnd as i64
    }
}

pub fn find_window(
    title: Option<&str>,
    pos: &Option<(i16, i16)>,
    size: &Option<(u16, u16)>,
) -> Option<i64> {
    let title = title.map(HSTRING::from);
    let left_top = pos.map(|p| (p.0 as i32, p.1 as i32));
    let mut info = FindWindowInfo {
        hwnd: HWND::default(),
        title: title.as_ref(),
        buffer: &mut vec![0u16; title.as_ref().map_or(0, |t| t.len() + 1)],
        left_top,
        right_bottom: size.map(|s| {
            let (x, y) = left_top.unwrap();
            (x + s.0 as i32, y + s.1 as i32)
        }),
        error: u32::MAX,
    };

    let _res = unsafe { EnumWindows(Some(find_window_proc), LPARAM(&mut info as *mut _ as _)) };
    if info.hwnd.is_invalid() {
        // not found
        return None;
    }

    // require strict match when we don't know the title
    if title.is_none() && info.error != 0 {
        return None;
    }

    Some(info.hwnd.0 as i64)
}

#[derive(Debug)]
struct FindWindowInfo<'a> {
    hwnd: HWND,
    title: Option<&'a HSTRING>,
    buffer: &'a mut [u16],
    left_top: Option<(i32, i32)>,
    right_bottom: Option<(i32, i32)>,
    error: u32
}

extern "system" fn find_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL { unsafe {
    let info = &mut *(lparam.0 as *mut FindWindowInfo);

    // check title or ignore if not specified
    if let Some(title) = info.title {
        let title_len = title.len();
        let len = GetWindowTextLengthW(hwnd) as usize;
        if len != title_len {
            return TRUE;
        }
        if title_len != 0 && GetWindowTextW(hwnd, info.buffer) == 0 {
            return TRUE;
        }
        if title.deref() != &info.buffer[..len] {
            return TRUE;
        }
    }

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    if let Some((left, top)) = info.left_top {
        let mut new_error = rect.left.abs_diff(left) + rect.top.abs_diff(top);

        if let Some((right, bottom)) = info.right_bottom {
            new_error += rect.right.abs_diff(right) + rect.bottom.abs_diff(bottom);
        }

        if new_error < info.error {
            info.hwnd = hwnd;
            info.error = new_error;
        }

        if new_error == 0 {
            return FALSE;
        }
    } else {
        // if the position is not specified, we can just take the first match
        info.hwnd = hwnd;
        info.error = 0;
        return FALSE;
    }

    let _ = EnumChildWindows(Some(hwnd), Some(find_window_proc), LPARAM(info as *mut _ as _));
    if info.error == 0 {
        return FALSE;
    }

    TRUE
} }