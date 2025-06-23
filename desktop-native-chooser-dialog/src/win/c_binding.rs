use crate::win::dialog::choose;
use crate::win::file_filter::WinFileFilter;
use crate::win::folders::load_folders;
use crate::win::params::{ChoosingMode, WinChooserDialogParams};
use crate::win::utils::ShellItemEx;
use crate::win::{mime, utils};
use libc::size_t;
use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::ptr::null_mut;
use windows::core::{GUID, HSTRING};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows::Win32::UI::Shell::IShellItem2;

#[repr(C)]
pub struct StringArray {
    pub paths: *mut *mut c_char,
    pub count: size_t,
}

impl StringArray {
    fn init(count: usize) -> *mut Self {
        unsafe {
            let array = libc::malloc(size_of::<StringArray>()) as *mut StringArray;
            
            let paths_ptr = libc::malloc(size_of::<*mut c_char>() * count) as *mut *mut c_char;
            
            (*array).paths = paths_ptr;
            (*array).count = count;
            
            array
        }
    }
    
    unsafe fn set(&self, index: usize, value: impl Into<Vec<u8>>) {
        unsafe {
            let c_string = CString::new(value).unwrap();
            *self.paths.add(index) = c_string.into_raw();
        }
    }
    
    unsafe fn set_all(&self, index: usize, values: impl Iterator<Item=impl Into<Vec<u8>>>) {
        unsafe {
            for (i, value) in values.enumerate() {
                self.set(index + i, value);
            }
        }
    }
    
    unsafe fn get(&self, index: usize) -> *mut c_char {
        unsafe {
            *self.paths.add(index)
        }
    }
}

#[repr(C)]
pub struct CResult<T> {
    pub err: *mut c_char,
    pub result: *mut T,
}

#[repr(C)]
pub struct FileFilter {
    pub name: *const c_char,
    pub patterns: *const c_char,
}

#[unsafe(no_mangle)]
pub extern "C" fn init_com() -> bool {
    utils::init_com(COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
}

#[unsafe(no_mangle)]
pub extern "C" fn quit_com() {
    utils::quit_com()
}

/// # Safety
/// Function expects a valid pointer to a Rust string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_string(string: *mut c_char) {
    if string.is_null() {
        return;
    }

    unsafe {
        let _ = CString::from_raw(string);
    }
}

/// # Safety
/// Function expects a valid pointer to a `StringArray`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_string_array(array: *mut StringArray) {
    if array.is_null() {
        return;
    }

    unsafe {
        for i in 0..(*array).count {
            free_string((*array).get(i));
        }
        libc::free((*array).paths as _);
        libc::free(array as _);
    }
}

/// # Safety
/// Function expects valid pointers to `title`, `filters`, `initial_directory` and `suggested_name`
/// C strings.
/// Also, a caller must free the returned `StringArray` using `free_string_array`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn show_dialog(
    id_part1: u64,
    id_part2: u64,
    title: *const c_char,
    filters: *const FileFilter,
    filters_count: size_t,
    mode: ChoosingMode,
    multiple: bool,
    initial_directory: *const c_char,
    suggested_name: *const c_char,
    owner: *const c_void,
) -> CResult<StringArray> { unsafe {
    let id = (id_part1 != 0 || id_part2 != 0).then(|| GUID::from_u128((id_part1 as u128) << 64 | (id_part2 as u128)));

    let title: Option<HSTRING> = (!title.is_null()).then(|| CStr::from_ptr(title).to_str().unwrap().into());

    let filters: Vec<WinFileFilter> = if !filters.is_null() && filters_count > 0 {
        (0..filters_count)
            .map(|i| {
                let filter = &*filters.add(i);

                let name = CStr::from_ptr(filter.name).to_str().unwrap();
                let patterns = CStr::from_ptr(filter.patterns).to_str().unwrap();

                WinFileFilter {
                    name: name.into(),
                    patterns: patterns.into(),
                }
            })
            .collect()
    } else {
        vec![]
    };

    let initial_directory: Option<HSTRING> = (!initial_directory.is_null()).then(|| CStr::from_ptr(initial_directory).to_str().unwrap().into());

    let suggested_name: Option<HSTRING> = (!suggested_name.is_null()).then(|| CStr::from_ptr(suggested_name).to_str().unwrap().into());

    let owner = (!owner.is_null()).then_some(HWND(owner as _));

    let params = WinChooserDialogParams {
        id,
        title,
        filters,
        mode,
        multiple,
        initial_directory,
        suggested_name,
        owner,
    };

    let result = choose(&params);

    match result {
        Ok((selection, folder)) => {
            let count = selection.len() + 1; // +1 for the folder
            let result = StringArray::init(count);
            
            if let Some(folder) = folder {
                let folder_str = folder.absolute_parsing_name().to_string();
                (*result).set(0, folder_str.as_str());
            }
            (*result).set_all(1, selection.into_iter().map(|item| item.absolute_parsing_name().to_string()));

            CResult {
                err: null_mut(),
                result,
            }
        }
        Err(err) => CResult {
            err: CString::new(err.message()).unwrap().into_raw(),
            result: null_mut(),
        },
    }
} }

/// # Safety
/// A caller must free the returned `StringArray` using `free_string_array`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn well_known_directories() -> *mut StringArray { unsafe {
    let result = load_folders();

    match result {
        Ok(folders) => {
            let array = StringArray::init(folders.len());
            (*array).set_all(0, folders.into_iter());
            array
        }
        Err(_) => ptr::null_mut(),
    }
} }

/// # Safety
/// Function expects a valid pointer to `extension` C string.
/// Also, a caller must free the returned C string using `free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn type_for_extension(extension: *const c_char) -> *mut c_char { unsafe {
    let extension = CStr::from_ptr(extension).to_str().unwrap();
    let result = mime::type_for_extension(extension);

    match result {
        Ok(Some(mime_type)) => CString::new(mime_type).unwrap().into_raw(),
        _ => ptr::null_mut(),
    }
} }


/// # Safety
/// Function expects valid pointers to `mime_type` and `mime_subtype` C strings.
/// Also, a caller must free the returned `StringArray` using `free_string_array`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn extensions_for_type(mime_type: *const c_char, mime_subtype: *const c_char) -> *mut StringArray { unsafe {
    let mime_type = CStr::from_ptr(mime_type).to_str().unwrap();
    let mime_subtype = CStr::from_ptr(mime_subtype).to_str().unwrap();
    let result = mime::extensions_for_type(mime_type, mime_subtype);

    match result {
        Ok(extensions) => {
            let array = StringArray::init(extensions.len());
            (*array).set_all(0, extensions.into_iter());
            array
        }
        Err(_) => ptr::null_mut(),
    }
} }

/// # Safety
/// Function expects a valid pointer to an `IShellItem2` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_item_size(item: *const c_void) -> i64 {
    if item.is_null() {
        return 0;
    }

    unsafe {
        match (*(item as *const IShellItem2)).size() {
            Ok(size) => size as i64,
            Err(_) => -1,
        }
    }
}

/// # Safety
/// Function expects a valid pointer to an `IShellItem2` object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_item_type(item: *const c_void) -> *mut c_char {
    if item.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        match (*(item as *const IShellItem2)).content_type() {
            Ok(mime) => CString::new(mime.as_bytes()).unwrap().into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}