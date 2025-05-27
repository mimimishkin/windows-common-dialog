use crate::java::awt_window::JAWT_DrawingSurfaceInfo;
use std::ffi::{c_int, c_ulong, c_void};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

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

pub fn find_window(
    title: Option<&str>,
    pos: &Option<(i16, i16)>,
    size: &Option<(u16, u16)>,
) -> Option<i64> {
    let is_x11 = std::env::var("DISPLAY").is_ok();
    if is_x11 {
        fn get_window_title<C: Connection>(conn: &C, window: Window) -> Option<String> {
            let reply = conn
                .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)
                .ok()?.reply().ok()?;
            String::from_utf8(reply.value).ok()
        }

        fn get_window_coords<C: Connection>(conn: &C, window: Window, parent: Window) -> Option<(i16, i16, u16, u16)> {
            let geom = conn.get_geometry(window).ok()?.reply().ok()?;
            let trans = conn.translate_coordinates(window, parent, 0, 0).ok()?.reply().ok()?;
            Some((trans.dst_x, trans.dst_y, geom.width, geom.height))
        }

        fn search_windows<C: Connection>(
            conn: &C,
            window: Window,
            title: Option<&str>,
            pos: &Option<(i16, i16)>,
            size: &Option<(u16, u16)>
        ) -> Option<(Window, u16)> {
            let tree = conn.query_tree(window).ok()?.reply().ok()?;

            let mut best: Option<Window> = None;
            let mut best_error = u16::MAX;

            for child in tree.children {
                // check title or ignore if not specified
                if let Some(title) = title {
                    let child_title = get_window_title(conn, child);
                    if child_title.is_none() || child_title.unwrap() != title {
                        continue;
                    }
                }

                if let Some((ref px, ref py)) = pos {
                    let (cx, cy, cw, ch) = get_window_coords(conn, child, window).unwrap_or_default();
                    let mut current_error = cx.abs_diff(px) + cy.abs_diff(py);

                    if let Some((ref sw, ref sh)) = size {
                        current_error += cw.abs_diff(sw) + ch.abs_diff(sh);
                    }

                    if current_error < best_error {
                        best = Some(child);
                        best_error = current_error;
                    }
                } else {
                    // if the position is not specified, we can just take the first match
                    return Some((child, 0));
                }

                if let Some(res) = search_windows(conn, child, title, pos, size) {
                    if res.1 == 0 {
                        return Some(res);
                    } else if res.1 < best_error {
                        best = Some(res.0);
                        best_error = res.1;
                    }
                }
            }

            best.map(|b| (b, best_error))
        }

        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];
        let (window, err) = search_windows(&conn, screen.root, title, pos, size)?;

        // require strict match when we don't know the title
        if title.is_none() && err != 0 {
            return None;
        }

        Some(window as i64)
    } else {
        // wayland doesn't provide info about other windows
        None
    }
}