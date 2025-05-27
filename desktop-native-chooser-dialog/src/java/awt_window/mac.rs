use crate::java::awt_window::JAWT_DrawingSurfaceInfo;
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

pub fn find_window(
    title: Option<&str>,
    pos: &Option<(i16, i16)>,
    size: &Option<(u16, u16)>,
) -> Option<i64> { unsafe {
    let window = with_main_thread(|mtm| {
        let title = title.as_ref().map(|t| NSString::from_str(&t));
        let windows = NSApplication::sharedApplication(mtm).windows();
        
        let pos = pos.map(|(x, y)| (x as f64, y as f64));
        let size = size.map(|(w, h)| (w as f64, h as f64));
        let res = find_window_with_nested(windows, title.as_deref(), &pos, &size);

        res.and_then(|(window, error)| {
            if title.is_some() || error < 0.1 { 
                Some(window)
            } else { 
                None
            }
        })
    }).flatten();

    let prt = window.map(|w| w.as_ref() as *const _).unwrap_or_default();
    if prt.is_null() {
        None
    } else {
        Some(prt as i64)
    }
} }

fn find_window_with_nested(
    windows: Retained<NSArray<NSWindow>>,
    title: Option<&NSString>,
    pos: &Option<(f64, f64)>,
    size: &Option<(f64, f64)>,
) -> Option<(Retained<NSWindow>, f64)> {
    let mut best = None;
    let mut best_error = f64::MAX;

    for window in windows {
        if let Some(ref title) = title {
            if unsafe { !window.title().isEqualToString(title) } {
                continue
            }
        }

        if let Some((ref px, ref py)) = pos {
            let bounds = window.frame();
            let cx = bounds.origin.x;
            let cy = bounds.origin.y;
            let mut error = (cx - px).abs() + (cy - py).abs();

            if let Some((ref sw, ref sh)) = size {
                let cw = bounds.size.width;
                let ch = bounds.size.height;
                error += (cw - sw).abs() + (ch - sh).abs();
            }

            if error < best_error {
                best = Some(window.clone());
                best_error = error;
            }
        } else { 
            // if the position is not specified, we can just take the first match
            return Some((window, 0.0));
        }

        if let Some(children) = unsafe { window.childWindows() } {
            if let Some(res) = find_window_with_nested(children, title, pos, size) {
                if res.1 < 0.1 {
                    return Some(res);
                } else if res.1 < best_error {
                    best = Some(res.0);
                    best_error = res.1;
                }
            }
        }
    }
    
    best.map(|b| (b, best_error))
}