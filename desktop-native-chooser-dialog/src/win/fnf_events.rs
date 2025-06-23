use crate::win::utils::{create_item_in, load_string, ShellItemEx, WinRes};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use windows::core::{implement, w, Interface, Ref, HSTRING, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_PATH_NOT_FOUND, HWND, LPARAM, LRESULT, S_OK, WPARAM};
use windows::Win32::System::Com::IServiceProvider;
use windows::Win32::System::Diagnostics::Debug::{FormatMessageW, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_ARGUMENT_ARRAY, FORMAT_MESSAGE_FROM_STRING};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_RETURN, VK_SHIFT};
use windows::Win32::UI::Shell::{DefSubclassProc, IFileDialog, IFileDialogEvents, IFileDialogEvents_Impl, IShellBrowser, IShellItem, IShellItemArray, SID_STopLevelBrowser, SetWindowSubclass, FDEOR_DEFAULT, FDESVR_DEFAULT, FDE_OVERWRITE_RESPONSE, FDE_SHAREVIOLATION_RESPONSE, SVGIO_SELECTION};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetWindowTextLengthW, GetWindowTextW, MessageBoxW, SetWindowTextW, BN_CLICKED, EN_CHANGE, IDOK, MB_ICONWARNING, MB_OK, WM_COMMAND};
use windows_core::imp::CoTaskMemFree;
use windows_core::PWSTR;

#[implement(IFileDialogEvents)]
pub struct FileAndFolderEvents {
    user_selection: *mut Vec<IShellItem>,
    last_folder: *mut IShellItem,

    last_selected: RefCell<Vec<IShellItem>>,
    last_active: RefCell<Option<IShellItem>>,
    editbox: RefCell<Option<HWND>>,
    editbox_text: RefCell<HSTRING>,
    window: RefCell<Option<HWND>>,
    was_my_change: RefCell<bool>,
    was_ok_clicked: RefCell<bool>,
    ok_text: RefCell<HSTRING>,
    open_text: RefCell<HSTRING>,
    select_folder_text: RefCell<HSTRING>,
    path_not_found_text: RefCell<HSTRING>,
    dialog_ptr: *const IFileDialog,
    is_initialized: RefCell<bool>,
}

impl<'a> FileAndFolderEvents {
    pub fn new(
        user_selection: &'a mut Vec<IShellItem>,
        last_folder: &'a mut IShellItem,
        dialog: &'a IFileDialog,
    ) -> Self {
        Self {
            user_selection,
            last_folder,
            last_selected: RefCell::default(),
            last_active: RefCell::default(),
            editbox: RefCell::default(),
            editbox_text: RefCell::default(),
            window: RefCell::default(),
            was_my_change: RefCell::default(),
            was_ok_clicked: RefCell::default(),
            ok_text: RefCell::default(),
            open_text: RefCell::default(),
            select_folder_text: RefCell::default(),
            path_not_found_text: RefCell::default(),
            dialog_ptr: dialog,
            is_initialized: RefCell::default(),
        }
    }
}

const DO_NOT_NOTIFY_SUBCLASS: usize = 1294;
extern "system" fn track_editbox(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    dwrefdata: usize,
) -> LRESULT { unsafe {
    if let WM_COMMAND = umsg {
        if EN_CHANGE == ((wparam.0 >> 16) & 0xFFFF) as u32 {
            let events = &*(dwrefdata as *const FileAndFolderEvents);
            if *events.was_my_change.borrow() {
                return LRESULT(0);
            } else {
                let dialog = &*events.dialog_ptr;
                if GetWindowTextLengthW(events.editbox.borrow().unwrap()) == 0 {
                    let _ = dialog.SetOkButtonLabel(&*events.select_folder_text.borrow());
                } else {
                    let _ = dialog.SetOkButtonLabel(events.ok_text.borrow().deref());
                }
            }
        }
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
} }

const TRACK_OK_CLICKS_SUBCLASS: usize = 5417;
extern "system" fn substitute_ok(
    hwnd: HWND,
    umsg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uidsubclass: usize,
    dwrefdata: usize,
) -> LRESULT { unsafe {
    if let WM_COMMAND = umsg {
        if BN_CLICKED == ((wparam.0 >> 16) & 0xFFFF) as u32 && IDOK.0 == (wparam.0 & 0xFFFF) as i32 {
            let events = &*(dwrefdata as *const FileAndFolderEvents);
            let dialog = &*(events.dialog_ptr);
            if dialog.GetFolder().and_then(|f| dialog.SetFolder(&f)).is_ok() {
                // We can't process closing the dialog here, dialog.Close just doesn't work for some reason. 
                // So we update the dialog folder and handle user selection in OnFolderChanging. 
                *events.was_ok_clicked.borrow_mut() = true;
            }
            return LRESULT(0);
        }
    }

    DefSubclassProc(hwnd, umsg, wparam, lparam)
} }

const LINK_SUFFIX: &[u16; 4] = &['.' as _, 'l' as _, 'n' as _, 'k' as _];

/// This event handler is used to allow the user to select files and directories simultaneously.
/// The OK button now always closes the dialog. Now the only way left to open the folder is to
/// double-click on it or press Enter when the folder is in focus.
///
/// It tries to behave like the native file dialog:
/// 1. Editbox is updated to contain the user's current selection (both files and dirs)
/// 2. Selected elements are ignored, only the names in the editbox are used to determine the user's choice
/// 3. Shift + Enter successfully ends the dialog with the current selection
///
/// And some additions specific to this type of the dialog:
/// 1. The name of the OK button now changes depending on the current selection - if it's a file 
///    list file, "Open" will be shown, if it's a directory list, "Select Folder" will be shown,
///    otherwise "OK" will be shown
/// 2. The title was changed to "Open File / Select Folder"
///
/// ## How it works
///
/// The first thing to do is populate the editbox with user selection, as the standard
/// implementation only does this for selected files, not directories. `pfd.GetSelectedItems` also
/// returns only selected files. So we'll use IShellView, which IFileDialog actually implements.
/// When manually updating the editbox, we will notice that the selection is reset. To solve this
/// problem, simply add a subclass to the editbox that ignores change messages.
///
/// Further, we will assume that the OK button can only try to either successfully end the
/// dialog or open the folder. In the first case, we simply parse names from the editbox and pass 
/// them to [FileAndFolderEvents::user_selection].
/// In the second, we will reject the folder changing and repeat steps from case 1. All we need to
/// know is whether the OK button has been clicked. To archive this, add a subclass to the dialog
/// window that tracks OK clicks.
#[allow(non_snake_case)]
impl IFileDialogEvents_Impl for FileAndFolderEvents_Impl {
    /// Checks filenames in editbox and forces dialog to close with S_OK
    fn OnFileOk(&self, pfd: Ref<'_, IFileDialog>) -> WinRes<()> { unsafe {
        let dialog = pfd.ok()?;
        let folder = dialog.GetFolder()?;

        let edit = self.editbox.borrow().unwrap();
        let len = GetWindowTextLengthW(edit);
        let mut names_inlined = vec![0u16; len as usize + 1];
        GetWindowTextW(edit, &mut names_inlined);
        let names_inlined = &names_inlined[..len as usize];

        let user_selection = &mut *(self.user_selection);
        user_selection.clear();

        let not_found = ERROR_PATH_NOT_FOUND.to_hresult();
        let get_item = |name: &HSTRING| -> WinRes<IShellItem> {
            let item = match create_item_in(&folder, name) {
                Ok(item) => item,
                Err(err) => {
                    if not_found == err.code() {
                        let message = {
                            let args = [names_inlined.as_ptr()];

                            let text = self.path_not_found_text.borrow();
                            let mut buffer: *mut u16 = std::ptr::null_mut::<u16>();
                            let res = FormatMessageW(
                                FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_STRING | FORMAT_MESSAGE_ARGUMENT_ARRAY,
                                Some(text.as_ptr() as *const _),
                                0,
                                0,
                                PWSTR(&mut buffer as *mut *mut u16 as *mut u16),
                                0,
                                Some(args.as_ptr() as *const _),
                            );
                            if res == 0 {
                                GetLastError().ok()?;
                            }

                            let message = std::slice::from_raw_parts(buffer, res as usize);
                            let message = HSTRING::from_wide(message);
                            CoTaskMemFree(buffer as _);
                            message
                        };

                        let window = *self.window.borrow();
                        let title = {
                            let title_len = GetWindowTextLengthW(window.unwrap());
                            let mut title = vec![0u16; title_len as usize + 1];
                            GetWindowTextW(window.unwrap(), &mut title[..]);
                            HSTRING::from_wide(&title[..title_len as usize])
                        };

                        MessageBoxW(window, &message, &title, MB_ICONWARNING | MB_OK);
                    }
                    return Err(err)
                }
            };



            let is_link = name.ends_with(LINK_SUFFIX);
            let resolved = if is_link {
                // here is possible to change the directory to 'The Internet' if the active item in
                // selection is an internet resource as standard implementation does. However, I 
                // think it's pointless

                item.link_target()?
            } else {
                item
            };

            Ok(resolved)
        };

        if names_inlined.is_empty() {
            user_selection.push(folder);
        } else {
            let many = names_inlined.contains(&('"' as u16));
            if many {
                let iter = names_inlined
                    .split(|&c| c == '"' as u16)
                    .filter(|s| s.iter().any(|&c| c != ' ' as u16));
                for name in iter {
                    let item = get_item(&HSTRING::from_wide(name))?;
                    user_selection.push(item);
                }
            } else {
                let item = get_item(&HSTRING::from_wide(names_inlined))?;
                user_selection.push(item);
            }
        }

        dialog.Close(S_OK)
    } }

    /// Prohibits changing the folder if the OK button was clicked or shift + enter was pressed
    fn OnFolderChanging(&self, pfd: Ref<'_, IFileDialog>, psifolder: Ref<'_, IShellItem>) -> WinRes<()> { unsafe {
        // it's weird, but the standard implementation updates the last folder even if this is the
        // user's selection, so we do the same
        *self.last_folder = psifolder.ok()?.to_owned();

        let shift_state = GetKeyState(VK_SHIFT.0 as i32) as u16;
        let shift_pressed = (shift_state & 0x8000) != 0;
        let enter_state = GetKeyState(VK_RETURN.0 as i32) as u16;
        let enter_pressed = (enter_state & 0x8000) != 0;

        if (shift_pressed && enter_pressed) || *self.was_ok_clicked.borrow() {
            *self.was_ok_clicked.borrow_mut() = false;
            return self.OnFileOk(pfd);
        } else if let Some(editbox) = *self.editbox.borrow() {
            SetWindowTextW(editbox, w!(""))?;
        }

        Ok(())
    } }

    fn OnFolderChange(&self, _pfd: Ref<'_, IFileDialog>) -> WinRes<()> {
        Ok(())
    }

    fn OnSelectionChange(&self, pfd: Ref<'_, IFileDialog>) -> WinRes<()> { unsafe {
        let dialog = pfd.ok()?;

        let service_provider = dialog.cast::<IServiceProvider>()?;
        let browser = service_provider.QueryService::<IShellBrowser>(&SID_STopLevelBrowser)?;
        let view = browser.QueryActiveShellView()?;

        let mut is_initialized = self.is_initialized.borrow_mut();
        if !*is_initialized {
            let mut buffer = [0u16; 256];
            let user_module = GetModuleHandleW(w!("user32.dll"))?;
            let ok_len = load_string(user_module, 800 /* OK */, &mut buffer)?;
            let ok = HSTRING::from_wide(&buffer[..ok_len]);
            dialog.SetOkButtonLabel(&ok)?;

            let dlg_module = GetModuleHandleW(w!("comdlg32.dll"))?;
            let open_len = load_string(dlg_module, 384 /* Open */, &mut buffer)?;
            let open = HSTRING::from_wide(&buffer[..open_len]);
            let select_folder_len = load_string(dlg_module, 439 /* Select Folder */, &mut buffer)?;
            let select_folder = HSTRING::from_wide(&buffer[..select_folder_len]);
            let path_not_found_len = load_string(dlg_module, 392 /* Path not found */, &mut buffer)?;
            let path_not_found = HSTRING::from_wide(&buffer[..path_not_found_len]);

            *self.ok_text.borrow_mut() = ok;
            *self.open_text.borrow_mut() = open;
            *self.select_folder_text.borrow_mut() = select_folder;
            *self.path_not_found_text.borrow_mut() = path_not_found;

            let window = browser.GetWindow()?;
            *self.window.borrow_mut() = Some(window);

            // track clicks on the OK button
            let this_ptr = &self.this as *const _ as _;
            SetWindowSubclass(window, Some(substitute_ok), TRACK_OK_CLICKS_SUBCLASS, this_ptr).ok()?;

            // forbid notifying the dialog about editbox changes to prevent selection reset
            let combo_ex = FindWindowExW(Some(window), None, w!("ComboBoxEx32"), None)?;
            let combo = FindWindowExW(Some(combo_ex), None, w!("ComboBox"), None)?;
            SetWindowSubclass(combo, Some(track_editbox), DO_NOT_NOTIFY_SUBCLASS, this_ptr).ok()?;
            *self.editbox.borrow_mut() = Some(combo_ex);

            let filename = dialog.GetFileName()?.to_hstring();
            let mut editbox_text = self.editbox_text.borrow_mut();
            *editbox_text = filename;

            *is_initialized = true;
        }

        let mut last_active = self.last_active.borrow_mut();
        let active = dialog.GetCurrentSelection()?;
        if last_active.as_ref().is_some_and(|last| *last == active) {
            // This method is usually invoked 2 times per selection.
            // To avoid wasting CPU time, inhibit repeated firings
            return Ok(());
        } else {
            *last_active = Some(active);
        }

        let mut last_selected = self.last_selected.borrow_mut();
        match view.GetItemObject::<IShellItemArray>(SVGIO_SELECTION) {
            Ok(array) => {
                last_selected.deref_mut().clear();
                let count = array.GetCount()?;
                for i in 0..count {
                    let item = array.GetItemAt(i as _)?;
                    last_selected.push(item);
                }
            },
            Err(_) => {
                last_selected.clear();
                return Ok(());
            }
        };

        let names = last_selected.iter().map(IShellItem::relative_editing_name).collect::<Vec<_>>();
        let names_inlined: HSTRING = match names.len() {
            1 => names.into_iter().next().unwrap(),
            _ => names.iter().map(|i| format!("\"{i}\" ")).collect::<String>().into(),
        };

        if last_selected.iter().all(|i| i.is_file().unwrap_or(false)) {
            dialog.SetOkButtonLabel(&*self.open_text.borrow())?;
        } else if last_selected.iter().all(|i| i.is_directory().unwrap_or(false)) {
            dialog.SetOkButtonLabel(&*self.select_folder_text.borrow())?;
        } else {
            dialog.SetOkButtonLabel(&*self.ok_text.borrow())?;
        }

        let mut editbox_text = self.editbox_text.borrow_mut();
        *editbox_text = names_inlined;
        *self.was_my_change.borrow_mut() = true;
        SetWindowTextW(self.editbox.borrow().unwrap(), PCWSTR(editbox_text.as_ptr()))?;
        *self.was_my_change.borrow_mut() = false;

        Ok(())
    } }

    fn OnShareViolation(&self, _pfd: Ref<'_, IFileDialog>, _psi: Ref<'_, IShellItem>) -> WinRes<FDE_SHAREVIOLATION_RESPONSE> {
        Ok(FDESVR_DEFAULT)
    }

    fn OnTypeChange(&self, _pfd: Ref<'_, IFileDialog>) -> WinRes<()> {
        Ok(())
    }

    fn OnOverwrite(&self, _pfd: Ref<'_, IFileDialog>, _psi: Ref<'_, IShellItem>) -> WinRes<FDE_OVERWRITE_RESPONSE> {
        Ok(FDEOR_DEFAULT)
    }
}