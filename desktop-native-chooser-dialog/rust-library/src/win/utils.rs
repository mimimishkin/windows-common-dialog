use crate::ChooserDialogError;
use std::ops::Deref;
pub use windows::core::Result as WinRes;
use windows::core::{BOOL, GUID, HRESULT, HSTRING, PWSTR};
use windows::Win32::Foundation::{ERROR_CANCELLED, ERROR_PATH_NOT_FOUND, HMODULE, HWND, LPARAM, RECT, S_FALSE, S_OK, TRUE};
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows::Win32::System::SystemServices::{SFGAO_FOLDER, SFGAO_STREAM};
use windows::Win32::UI::Shell::{BHID_EnumItems, BHID_SFUIObject, IEnumShellItems, IShellItem, IShellLinkW, PathIsRelativeW, SHCreateItemFromParsingName, SHCreateShellItem, SHGetKnownFolderPath, KF_FLAG_DEFAULT, SIGDN, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_PARENTRELATIVE, SIGDN_PARENTRELATIVEEDITING};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, LoadStringW};

pub trait ExtractName {
    fn get_display_name(&self, name_type: SIGDN) -> WinRes<HSTRING>;
    
    fn relative_name(&self) -> HSTRING {
        self.get_display_name(SIGDN_PARENTRELATIVE).expect("IShellItem must provide SIGDN_PARENTRELATIVE")
    }
    
    fn relative_editing_name(&self) -> HSTRING {
        self.get_display_name(SIGDN_PARENTRELATIVEEDITING).expect("IShellItem must provide SIGDN_PARENTRELATIVEEDITING")
    }
    
    fn absolute_parsing_name(&self) -> HSTRING {
        self.get_display_name(SIGDN_DESKTOPABSOLUTEPARSING).expect("IShellItem must provide SIGDN_DESKTOPABSOLUTEPARSING")
    }
}

impl ExtractName for IShellItem {
    fn get_display_name(&self, name_type: SIGDN) -> WinRes<HSTRING> { unsafe {
        let name = self.GetDisplayName(name_type)?;
        let string = HSTRING::from_wide(name.as_wide());
        CoTaskMemFree(Some(name.as_ptr() as _));
        Ok(string)
    }
} }

pub const HRESULT_CANCELLED: HRESULT = ERROR_CANCELLED.to_hresult();

pub fn with_com<T, F: FnOnce() -> WinRes<T>>(f: F) -> WinRes<T> { unsafe {
    let init_com = |dw_co_init: COINIT| -> bool {
        match CoInitializeEx(None, dw_co_init) {
            // Successful local initialization (S_OK) or was already initialized
            // (S_FALSE) but still needs uninit
            S_OK | S_FALSE => true,

            // COM was already initialized with a different threading model
            HRESULT_CANCELLED => false,

            // Any other result is impossible
            hr => panic!("Failed to initialize COM. CoInitializeEx returned {hr}")
        }
    };

    let need_quit = init_com(COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    let result = f();
    if need_quit { 
        CoUninitialize(); 
    }
    
    result
} }

pub fn create_item(absolute_parsing_name: &HSTRING) -> WinRes<IShellItem> {
    let item: IShellItem = unsafe { SHCreateItemFromParsingName(absolute_parsing_name, None) }?;
    Ok(item)
}

pub fn travel_to_item(from: &IShellItem, dest: &[u16]) -> WinRes<IShellItem> { unsafe {
    dest.split(|char| *char == '\\' as u16).try_fold(from.to_owned(), |acc, part| {
        if part == ['.' as u16] { 
            return Ok(acc)
        } else if part == ['.' as u16, '.' as u16] {
            return acc.GetParent()
        }
        
        acc
            .iter_children()?
            .find(|item| item.relative_name().deref() == part || item.relative_editing_name().deref() == part)
            .ok_or(ERROR_PATH_NOT_FOUND.into())
    })
} }

pub fn create_item_in(folder: &IShellItem, name_or_path: &HSTRING) -> WinRes<IShellItem> { unsafe {
    if !PathIsRelativeW(name_or_path).as_bool() {
        let item = create_item(name_or_path)?;
        return (item.absolute_parsing_name() == *name_or_path)
            .then_some(item)
            .ok_or(ERROR_PATH_NOT_FOUND.into());
    }
    
    travel_to_item(folder, name_or_path)
} }

pub struct ChildrenIter {
    enum_items: IEnumShellItems,
}

impl Iterator for ChildrenIter {
    type Item = IShellItem;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let fetched = &mut [None; 1];
            self.enum_items.Next(fetched, None).ok()?;
            fetched[0].take()
        }
    }
}

pub trait IterChildren {
    fn iter_children(&self) -> WinRes<ChildrenIter>;
}

impl IterChildren for IShellItem {
    fn iter_children(&self) -> WinRes<ChildrenIter> {
        let enumerate: IEnumShellItems = unsafe { self.BindToHandler(None, &BHID_EnumItems) }?;
        Ok(ChildrenIter { enum_items: enumerate })
    }
}

pub fn load_string(module: HMODULE, id: u32, buffer: &mut [u16]) -> WinRes<usize> {
    let pwstr = PWSTR(buffer.as_mut_ptr());
    let res = unsafe { LoadStringW(Some(module.into()), id, pwstr, buffer.len() as _) };
    match res {
        -1 | 0 => Err(windows::core::Error::from_win32()),
        len => Ok(len as usize)
    }
}

pub trait LinkTarget {
    fn link_target(&self) -> WinRes<IShellItem>;
}

impl LinkTarget for IShellItem {
    fn link_target(&self) -> WinRes<IShellItem> { unsafe {
        let link: IShellLinkW = self.BindToHandler(None, &BHID_SFUIObject)?;
        let target_id = link.GetIDList()?;
        let target = SHCreateShellItem(None, None, target_id)?;
        Ok(target)
    }
} }

pub trait TypedItem {
    fn is_file(&self) -> WinRes<bool>;
    
    fn is_directory(&self) -> WinRes<bool>;
}

impl TypedItem for IShellItem {
    fn is_file(&self) -> WinRes<bool> {
        unsafe { Ok(self.GetAttributes(SFGAO_STREAM)?.contains(SFGAO_STREAM)) }
    }

    fn is_directory(&self) -> WinRes<bool> {
        unsafe { Ok(self.GetAttributes(SFGAO_FOLDER)?.contains(SFGAO_FOLDER)) }
    }
}

impl From<windows::core::Error> for ChooserDialogError {
    fn from(err: windows::core::Error) -> Self {
        ChooserDialogError::Generic(err.message())
    }
}

#[derive(Debug)]
struct FindWindowInfo<'a> {
    hwnd: HWND,
    title: Option<&'a HSTRING>,
    buffer: &'a mut [u16],
    coords: (i32, i32, i32, i32),
    error: u32
}

extern "system" fn find_window_with_proc(hwnd: HWND, lparam: LPARAM) -> BOOL { unsafe {
    let info = &mut *(lparam.0 as *mut FindWindowInfo);
    
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

    let target = info.coords;
    let new_error = rect.left.abs_diff(target.0) + rect.top.abs_diff(target.1) + rect.right.abs_diff(target.2) + rect.bottom.abs_diff(target.3);
    if new_error < info.error { 
        info.hwnd = hwnd;
        info.error = new_error;
    }
    
    TRUE
} }

pub fn find_window_with(title: Option<&HSTRING>, x: i32, y: i32, width: i32, height: i32) -> WinRes<HWND> {
    let mut info = FindWindowInfo {
        hwnd: HWND::default(),
        title,
        buffer: &mut vec![0u16; title.map_or(0, |t| t.len() + 1)],
        coords: (x, y, x + width, y + height),
        error: u32::MAX
    };
    
    let res = unsafe { EnumWindows(Some(find_window_with_proc), LPARAM(&mut info as *mut _ as _)) };
    if info.hwnd.is_invalid() { 
        // not found
        return Err(res.unwrap_err());
    }
    
    // require strict match when we don't know the title
    if title.is_none() && info.error != 0 {
        return Err(res.unwrap_err());
    }
    
    Ok(info.hwnd)
}

pub fn known_folder_path(id: &GUID) -> WinRes<String> { unsafe {
    let path = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)?;
    let str = path.to_string();
    CoTaskMemFree(Some(path.as_ptr() as _));
    Ok(str?)
} }