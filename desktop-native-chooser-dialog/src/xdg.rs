use crate::{ChooserDialogError, ChooserDialogOwner, ChooserDialogParams, ChooserDialogResult, ChooserRes, ChoosingMode};
use ashpd::desktop::file_chooser::*;
use ashpd::WindowIdentifier;
use std::ffi::CStr;
use std::sync::Arc;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};
use x11rb::rust_connection::RustConnection;
use ChoosingMode::*;

impl Into<FileFilter> for &crate::FileFilter{
    fn into(self) -> FileFilter {
        let (name, elements) = self.split();
        let name = name.unwrap_or_default(); // todo: default names
        elements.split('\0').fold(FileFilter::new(name), |acc, i| {
            if i.contains('/') {
                acc.mimetype(i)
            } else {
                acc.glob(i)
            }
        })
    }
}

impl From<ashpd::Error> for ChooserDialogError {
    fn from(err: ashpd::Error) -> Self {
        ChooserDialogError::Generic(err.to_string())
    }
}

fn find_window(title: Option<&str>, coords: &(i16, i16, u16, u16)) -> Option<WindowIdentifier> {
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
            target: &(i16, i16, u16, u16)
        ) -> Option<(Window, u16)> {
            let tree = conn.query_tree(window).ok()?.reply().ok()?;

            let mut best: Option<Window> = None;
            let mut best_error = u16::MAX;

            for child in tree.children {
                // check title or ignore
                if let Some(title) = title {
                    let child_title = get_window_title(conn, child);
                    if child_title.is_none() || child_title.unwrap() != title {
                        continue;
                    }
                }

                let (cx, cy, cw, ch) = get_window_coords(conn, child, window).unwrap_or_default();
                let current_error = cx.abs_diff(target.0) + cy.abs_diff(target.1) + cw.abs_diff(target.2) + ch.abs_diff(target.3);
                if current_error < best_error {
                    best = Some(child);
                    best_error = current_error;
                }

                if let Some((win, err)) = search_windows(conn, child, title, target) {
                    if err < best_error {
                        best = Some(win);
                        best_error = err;
                    }
                }
            }

            best.map(|b| (b, best_error))
        }

        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];
        let (window, err) = search_windows(&conn, screen.root, title, &coords)?;

        // require strict match when we don't know the title
        if title.is_none() && err != 0 {
            return None;
        }

        Some(WindowIdentifier::from_xid(window as _))
    } else { 
        // wayland doesn't provide info about other windows
        None
    }
}

pub fn choose(params: Arc<ChooserDialogParams>, callback: impl FnOnce(ChooserRes<ChooserDialogResult>) + Send + 'static) {
    async_std::task::spawn(async move {
        let owner = match params.owner {
            ChooserDialogOwner::X11 { window: id } => {
                Some(WindowIdentifier::from_xid(id))
            }
            ChooserDialogOwner::Wayland { surface_ptr, display_ptr } => unsafe {
                WindowIdentifier::from_wayland_raw(surface_ptr, display_ptr).await
            }
            ChooserDialogOwner::Desc { title, x, y, width, height } => {
                let title = (!title.is_null()).then(|| CStr::from_ptr(title).to_str().ok()).flatten();
                let coords = (x as i16, y as i16, width as u16, height as u16);
                find_window(title, &coords)
            },
            _ => None
        };
        let request = match params.mode {
            Saving => {
                SaveFileRequest::default()
                    .identifier(owner)
                    .title(params.title.as_ref().map(String::as_str))
                    .filters(params.filters.iter().map(|f| f.into()))
                    .current_folder::<&String>(params.initial_directory.as_ref())
                    .expect("Failed specify initial directory")
                    .current_name(params.suggested_name.as_ref().map(String::as_str))
                    .send().await
            }

            FilesOnly | DirectoriesOnly => {
                OpenFileRequest::default()
                    .identifier(owner)
                    .title(params.title.as_ref().map(String::as_str))
                    .filters(params.filters.iter().map(|f| f.into()))
                    .directory(params.mode == DirectoriesOnly)
                    .multiple(params.multiple)
                    .current_folder::<&String>(params.initial_directory.as_ref())
                    .expect("Failed specify initial directory")
                    .send().await
            }

            FilesAndDirectories => {
                panic!("FilesAndDirectories mode is not supported")
            }
        };
        
        let response = request.and_then(|r| r.response());
        let uris = response.map(|r| r.uris().iter().map(|i| i.to_string()).collect());
        let result = uris.map(|uris| ChooserDialogResult::new(uris, None));
        callback(result.map_err(|e| e.into()));
    });
}