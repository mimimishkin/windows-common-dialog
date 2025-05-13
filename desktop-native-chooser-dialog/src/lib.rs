//! This module provides a cross-platform file chooser dialog interface.
//! It shouldn't be used directly, it's only for outer kotlin project.

use itertools::Itertools;
use std::fmt::{Display, Formatter};
use std::iter::once;
use std::mem::ManuallyDrop;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum ChoosingMode {
    Saving,
    FilesOnly,
    FilesAndDirectories,
    DirectoriesOnly,
}

/// Inlined file filter. 
/// Filter name and elements are separated by '\0'.
/// For example, `Documents (*.txt, *.doc, *.docx)` becomes `"Documents\0*.txt\0*.doc\0*.docx"`
#[derive(Debug, Clone)]
pub struct FileFilter<'a>(&'a str);

impl FileFilter<'_> {
    pub fn split(&self) -> (Option<&str>, &str) {
        let (name, patterns) = self.0.split_once('\0').expect("Wrong filter format");
        ((!name.is_empty()).then_some(name), patterns)
    }
    
    pub fn name(&self) -> Option<&str> {
        let name = self.0.split_once('\0').expect("Wrong filter format").0;
        (!name.is_empty()).then_some(name)
    }

    pub fn elements_inlined(&self) -> &str {
        self.0.split_once('\0').expect("Wrong filter format").1
    }
    
    pub fn elements(&self) -> impl Iterator<Item=&str> {
        self.0.split('\0').skip(1)
    }
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub enum ChooserDialogOwner {
    HWND { ptr: *mut core::ffi::c_void },
    X11 { window: u64 },
    Wayland { surface_ptr: *mut core::ffi::c_void, display_ptr: *mut core::ffi::c_void },
    NSWindow { ptr: *const core::ffi::c_void },
    Desc { title: *const core::ffi::c_char, x: i32, y: i32, width: i32, height: i32 },
    #[default]
    None
}

unsafe impl Sync for ChooserDialogOwner {}
unsafe impl Send for ChooserDialogOwner {}

#[derive(Debug, Clone)]
pub struct ChooserDialogParams<'a> {
    pub id: Option<u128>,
    pub title: Option<&'a str>,
    pub filters: Vec<FileFilter<'a>>,
    pub mode: ChoosingMode,
    pub multiple: bool,
    pub initial_directory: Option<&'a str>,
    pub suggested_name: Option<&'a str>,
    pub owner: ChooserDialogOwner,
}

#[derive(Debug, Clone)]
pub struct ChooserDialogResult(String);

impl ChooserDialogResult {
    pub fn new(selected: Vec<String>, last_folder: Option<String>) -> Self {
        Self(last_folder.unwrap_or_default() + "\0" + &selected.join("\0"))
    }
}

#[derive(Debug, Clone)]
pub enum ChooserDialogError {
    Generic(String),
}

impl Display for ChooserDialogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChooserDialogError::Generic(msg) => write!(f, "ChooserDialogError: {msg}"),
        }
    }
}

type ChooserRes<T> = Result<T, ChooserDialogError>;

#[cfg(target_os = "windows")]
mod win;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod xdg;
// #[cfg(target_os = "macos")]
// mod mac;

// #[cfg(target_os = "macos")]
// use mac::*;
#[cfg(target_os = "windows")]
use win::*;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use xdg::*;

#[repr(C)]
pub struct SizedStrPtr {
    pub ptr: *const u8,
    pub len: usize
}

impl From<&str> for SizedStrPtr {
    fn from(value: &str) -> Self {
        SizedStrPtr {
            ptr: value.as_ptr(),
            len: value.len()
        }
    }
}

#[repr(C)]
pub struct FFIResult {
    pub is_ok: bool,
    pub result_or_message: SizedStrPtr
}

impl From<ChooserRes<String>> for FFIResult {
    fn from(value: ChooserRes<String>) -> Self {
        FFIResult {
            is_ok: value.is_ok(),
            result_or_message: {
                let string = match value {
                    Ok(string) => {
                        string
                    }
                    Err(err) => {
                        err.to_string()
                    }
                };
                SizedStrPtr::from(ManuallyDrop::new(string).as_str())
            }
        }
    }
}

/// # Safety
/// * [title] must be either a valid pointer to utf8 string without a trailing null, or null.
/// * [title_size] must contain the size of [title] in bytes or -1.
/// * The same with [initial_directory] and [suggested_name].
/// * [filters] must be either a valid pointer to an array of utf8 strings without a trailing null
///   terminated by '\0\0', or null.
/// * [filters_size] must contain the size of [filters] in bytes or -1.
/// * Caller takes care about freeing the result via [free_ffi_string].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn choose_ffi(
    id_part1: u64,
    id_part2: u64,
    title: *const std::ffi::c_uchar,
    title_size: i64,
    filters: *const std::ffi::c_uchar,
    filters_size: i64,
    mode: ChoosingMode,
    multiple: bool,
    initial_directory: *const std::ffi::c_uchar,
    initial_directory_size: i64,
    suggested_name: *const std::ffi::c_uchar,
    suggested_name_size: i64,
    owner: ChooserDialogOwner,
    callback: extern "C" fn(FFIResult),
) { unsafe {
    let id = ((id_part1 as u128) << 64) | (id_part2 as u128);
    let id = (id != 0).then_some(id);
    let title = (title_size != -1).then(|| std::slice::from_raw_parts(title, title_size as usize));
    let title = title.map(|t| std::str::from_utf8(t).unwrap());
    let filters = (filters_size != -1).then(|| std::slice::from_raw_parts(filters, filters_size as usize));
    let filters = filters.map(|f| std::str::from_utf8(f).unwrap());
    let filters = filters.map(|f| f.split("\0\0").map(FileFilter).collect::<Vec<_>>()).unwrap_or_default();
    let initial_directory = (initial_directory_size != -1).then(|| std::slice::from_raw_parts(initial_directory, initial_directory_size as usize));
    let initial_directory = initial_directory.map(|d| std::str::from_utf8(d).unwrap());
    let suggested_name = (suggested_name_size != -1).then(|| std::slice::from_raw_parts(suggested_name, suggested_name_size as usize));
    let suggested_name = suggested_name.map(|d| std::str::from_utf8(d).unwrap());

    let params = ChooserDialogParams {
        id,
        title,
        filters,
        mode,
        multiple,
        initial_directory,
        suggested_name,
        owner,
    };
    
    choose(Arc::new(params), move |result| {
        callback(FFIResult::from(result.map(|r| r.0)))
    });
} }

/// # Safety
/// Caller takes care about freeing the result via [free_ffi_string].
#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_mime_table_ffi() -> FFIResult {
    let mime_table_inlined = load_mime_table().map(|mime_table| {
        mime_table
            .into_iter()
            .map(|(mime, ext)| itertools::chain(once(mime), ext).join("\0"))
            .join("\0\0")
    });
    
    FFIResult::from(mime_table_inlined)
}

/// # Safety
/// Caller takes care about freeing the result via [free_ffi_string].
#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_folder_ffi() -> FFIResult {
    FFIResult::from(load_folders().map(|f| f.join("\0")))
}

/// # Safety
/// [string] must contain a valid pointer obtained from Rust String.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_ffi_string(string: SizedStrPtr) {
    unsafe {
        String::from_raw_parts(string.ptr as _, string.len, string.len);
    }
}

/// Can be used to check if the library is available on the current platform.
#[unsafe(no_mangle)]
pub extern "C" fn check_available() -> bool {
    true
}

#[cfg(feature = "awt_window")]
mod awt_window;

#[cfg(feature = "awt_window")]
use jni::{objects::{JClass, JObject}, sys::jlong, JNIEnv};

#[cfg(feature = "awt_window")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_binding_DesktopNativeChooserDialogH_getNativeHandle<'a>(
    mut env: JNIEnv<'a>,
    _: JClass<'a>,
    input: JObject<'a>
) -> jlong {
    let (err, handle) = awt_window::find_handle(&env, input);
    if let Some(message) = err {
        let _ = env.throw_new("java/lang/IllegalArgumentException", message);
    }
    handle
}

#[cfg(test)]
mod tests {
    use crate::{choose, ChooserDialogOwner, ChooserDialogParams, ChoosingMode, FileFilter};
    use std::sync::Arc;

    #[test]
    fn test_open_file_save_dialog() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Save File"),
            filters: vec![
                FileFilter("Text Files\0*.txt"),
                FileFilter("All Files\0*.*")
            ],
            mode: ChoosingMode::Saving,
            multiple: false,
            initial_directory: None,
            suggested_name: Some("document.txt"),
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_file_chooser() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Select a File"),
            filters: vec![
                FileFilter("All Files\0*.*"),
                FileFilter("Documents\0*.txt\0*.docx\0*.doc")
            ],
            mode: ChoosingMode::FilesOnly,
            multiple: false,
            initial_directory: None,
            suggested_name: None,
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_directory_chooser() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Select a Directory"),
            filters: vec![],
            mode: ChoosingMode::DirectoriesOnly,
            multiple: true,
            initial_directory: None,
            suggested_name: None,
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_file_or_directory_chooser() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Select a Directory"),
            filters: vec![
                FileFilter("All Files\0*.*"),
                FileFilter("Documents\0*.txt\0*.docx\0*.doc")
            ],
            mode: ChoosingMode::FilesAndDirectories,
            multiple: true,
            initial_directory: None,
            suggested_name: None,
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }
}