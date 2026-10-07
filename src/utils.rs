//! Low-level helpers: COM lifetime management and `IShellItem` utilities.

use std::ops::Deref;

use windows::Win32::Foundation::{
    ERROR_CANCELLED, ERROR_PATH_NOT_FOUND, HWND, RPC_E_CHANGED_MODE, S_FALSE, S_OK,
};
use windows::Win32::Storage::EnhancedStorage::{PKEY_ContentType, PKEY_Size};
use windows::Win32::System::Com::{
    COINIT, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoTaskMemFree,
    CoUninitialize,
};
use windows::Win32::System::SystemServices::{SFGAO_FOLDER, SFGAO_STREAM};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, BHID_SFUIObject, IEnumShellItems, IShellItem, IShellItem2, IShellLinkW,
    KF_FLAG_DEFAULT, PathIsRelativeW, SHCreateItemFromParsingName, SHCreateShellItem,
    SHGetKnownFolderPath, SIGDN, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_PARENTRELATIVE,
    SIGDN_PARENTRELATIVEEDITING,
};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};
pub use windows::core::Result as WinRes;
use windows::core::{GUID, HRESULT, HSTRING, Interface};

/// The `HRESULT` returned when the user cancels a dialog.
pub const HRESULT_CANCELLED: HRESULT = ERROR_CANCELLED.to_hresult();

/// Convenience methods on [`IShellItem`].
pub trait ShellItemEx {
    /// Returns the display name of this item in the requested format.
    fn get_display_name(&self, name_type: SIGDN) -> WinRes<HSTRING>;

    /// The name of the item within its parent folder (e.g. `"report.pdf"`).
    fn relative_name(&self) -> HSTRING {
        self.get_display_name(SIGDN_PARENTRELATIVE)
            .expect("IShellItem must provide SIGDN_PARENTRELATIVE")
    }

    /// Like [`relative_name`](Self::relative_name), but in a form that can be edited back
    /// into the dialog (e.g. a shortcut's target name).
    fn relative_editing_name(&self) -> HSTRING {
        self.get_display_name(SIGDN_PARENTRELATIVEEDITING)
            .expect("IShellItem must provide SIGDN_PARENTRELATIVEEDITING")
    }

    /// An absolute path/parsing name of the item (e.g. `"C:\Users\me\report.pdf"`).
    fn absolute_parsing_name(&self) -> HSTRING {
        self.get_display_name(SIGDN_DESKTOPABSOLUTEPARSING)
            .expect("IShellItem must provide SIGDN_DESKTOPABSOLUTEPARSING")
    }

    /// Whether this item is a file.
    fn is_file(&self) -> WinRes<bool>;

    /// Whether this item is a folder.
    fn is_directory(&self) -> WinRes<bool>;

    /// The size of this item in bytes.
    fn size(&self) -> WinRes<u64>;

    /// The MIME content type of this item.
    fn content_type(&self) -> WinRes<String>;

    /// Iterator over the direct children of this folder.
    fn iter_children(&self) -> WinRes<ChildrenIter>;

    /// The target of this shortcut/link item.
    fn link_target(&self) -> WinRes<IShellItem>;
}

/// Iterator over the items directly inside a folder.
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

impl ShellItemEx for IShellItem {
    fn get_display_name(&self, name_type: SIGDN) -> WinRes<HSTRING> {
        unsafe {
            let name = self.GetDisplayName(name_type)?;
            let string = HSTRING::from_wide(name.as_wide());
            CoTaskMemFree(Some(name.as_ptr() as _));
            Ok(string)
        }
    }

    fn is_file(&self) -> WinRes<bool> {
        unsafe { Ok(self.GetAttributes(SFGAO_STREAM)?.contains(SFGAO_STREAM)) }
    }

    fn is_directory(&self) -> WinRes<bool> {
        unsafe { Ok(self.GetAttributes(SFGAO_FOLDER)?.contains(SFGAO_FOLDER)) }
    }

    fn size(&self) -> WinRes<u64> {
        unsafe {
            let item: IShellItem2 = self.cast()?;
            item.GetUInt64(&PKEY_Size)
        }
    }

    fn content_type(&self) -> WinRes<String> {
        unsafe {
            let item: IShellItem2 = self.cast()?;
            Ok(item.GetString(&PKEY_ContentType)?.to_string()?)
        }
    }

    fn iter_children(&self) -> WinRes<ChildrenIter> {
        let enumerate: IEnumShellItems = unsafe { self.BindToHandler(None, &BHID_EnumItems) }?;
        Ok(ChildrenIter {
            enum_items: enumerate,
        })
    }

    fn link_target(&self) -> WinRes<IShellItem> {
        unsafe {
            let link: IShellLinkW = self.BindToHandler(None, &BHID_SFUIObject)?;
            let target_id = link.GetIDList()?;
            SHCreateShellItem(None, None, target_id)
        }
    }
}

/// Initializes COM on the current thread.
///
/// Returns `Ok(true)` if the caller must balance the call with [`quit_com`]: both `S_OK`
/// (fresh initialization) and `S_FALSE` (already initialized) still have to be matched by
/// [`CoUninitialize`]. Returns `Ok(false)` when COM was already initialized with a
/// *different* threading model ([`RPC_E_CHANGED_MODE`]); in that case the existing
/// apartment is reused and must *not* be balanced here.
pub fn init_com(coinit: COINIT) -> WinRes<bool> {
    unsafe {
        match CoInitializeEx(None, coinit) {
            S_OK | S_FALSE => Ok(true),
            RPC_E_CHANGED_MODE => Ok(false),
            hr => Err(hr.into()),
        }
    }
}

/// Undoes an [`init_com`] call that returned `Ok(true)`.
pub fn quit_com() {
    unsafe { CoUninitialize() }
}

/// Runs `f` with COM initialized as an STA on the current thread.
///
/// Correctly balances the COM initialization when `f` returns *and* when it panics.
pub fn with_com<T>(f: impl FnOnce() -> WinRes<T>) -> WinRes<T> {
    /// Releases the COM initialization on drop.
    struct Guard {
        balanced: bool,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if self.balanced {
                unsafe { CoUninitialize() }
            }
        }
    }

    let balanced = init_com(COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)?;
    let guard = Guard { balanced };
    let result = f();
    drop(guard);
    result
}

/// Creates a shell item from an absolute parsing name (e.g. `C:\Users\me` or a GUID path).
pub fn create_item(absolute_parsing_name: &HSTRING) -> WinRes<IShellItem> {
    unsafe { SHCreateItemFromParsingName(absolute_parsing_name, None) }
}

/// A `\\` character as used in dialog editboxes.
const PATH_SEPARATOR: u16 = b'\\' as u16;
const CURRENT_DIR: &[u16] = &[b'.' as u16];
const PARENT_DIR: &[u16] = &[b'.' as u16, b'.' as u16];

/// Walks `destination` as a relative path starting from `from`, following `.` and `..`
/// segments the way the shell does.
pub fn travel_to_item(from: &IShellItem, destination: &[u16]) -> WinRes<IShellItem> {
    destination
        .split(|&c| c == PATH_SEPARATOR)
        .try_fold(from.to_owned(), |folder, segment| {
            if segment == CURRENT_DIR {
                Ok(folder)
            } else if segment == PARENT_DIR {
                unsafe { folder.GetParent() }
            } else {
                folder
                    .iter_children()?
                    .find(|item| {
                        item.relative_name().deref() == segment
                            || item.relative_editing_name().deref() == segment
                    })
                    .ok_or(ERROR_PATH_NOT_FOUND.into())
            }
        })
}

/// Creates a shell item from `name_or_path`, resolving it relative to `folder` when it is
/// not an absolute path.
///
/// Errors with `ERROR_PATH_NOT_FOUND` if the name cannot be found.
pub fn create_item_in(folder: &IShellItem, name_or_path: &HSTRING) -> WinRes<IShellItem> {
    if !unsafe { PathIsRelativeW(name_or_path) }.as_bool() {
        let item = create_item(name_or_path)?;
        return (item.absolute_parsing_name() == *name_or_path)
            .then_some(item)
            .ok_or(ERROR_PATH_NOT_FOUND.into());
    }

    travel_to_item(folder, name_or_path)
}

/// Reads the text of a window or control.
pub fn window_text(hwnd: HWND) -> HSTRING {
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if length <= 0 {
        return HSTRING::new();
    }

    let mut buffer = vec![0u16; length as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    HSTRING::from_wide(&buffer[..copied.max(0) as usize])
}

/// The filesystem path of a known folder (e.g. `FOLDERID_Downloads`).
pub fn known_folder_path(id: &GUID) -> WinRes<String> {
    unsafe {
        let path = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)?;
        let text = path.to_string()?;
        CoTaskMemFree(Some(path.as_ptr() as _));
        Ok(text)
    }
}
