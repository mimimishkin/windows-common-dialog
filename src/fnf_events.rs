//! An [`IFileDialogEvents`] implementation that lets the user pick files *and* folders at
//! the same time, something the standard dialog cannot do on its own.
//!
//! # Behavior
//!
//! * The OK button always accepts the selection. To open a folder the user has to
//!   double-click it or press Enter while it is focused.
//! * The "file name" editbox mirrors the current selection, files and folders alike.
//! * Only the names written into the editbox decide what gets returned.
//! * Shift+Enter ends the dialog with the current selection.
//! * The OK button is relabeled depending on the selection: "Open" for files only,
//!   "Select Folder" for folders only, "OK" for a mixture.
//!
//! # How it works
//!
//! `IFileDialog::GetSelectedItems` only reports selected *files*, so the actual selection
//! is read from the underlying [`IShellView`] via `SVGIO_SELECTION`. Rewriting the editbox
//! would make the dialog reset the selection, so the editbox is subclassed
//! ([`track_editbox`]) to swallow the change notifications we generate ourselves.
//!
//! `Close` does not work from the OK button handler, so the OK button is subclassed
//! ([`substitute_ok`]) to force a folder change instead. The change is then rejected in
//! [`FileAndFolderEvents_Impl::OnFolderChanging`], which runs the real `OnFileOk`.

use std::cell::{Cell, RefCell};
use std::sync::OnceLock;

use windows::Win32::Foundation::{ERROR_PATH_NOT_FOUND, HWND, LPARAM, LRESULT, S_OK, WPARAM};
use windows::Win32::System::Com::IServiceProvider;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VIRTUAL_KEY, VK_RETURN, VK_SHIFT};
use windows::Win32::UI::Shell::{
    DefSubclassProc, FDE_OVERWRITE_RESPONSE, FDE_SHAREVIOLATION_RESPONSE, FDEOR_DEFAULT,
    FDESVR_DEFAULT, IFileDialog, IFileDialogEvents, IFileDialogEvents_Impl, IShellBrowser,
    IShellItem, IShellItemArray, SID_STopLevelBrowser, SVGIO_SELECTION, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, EN_CHANGE, FindWindowExW, GetWindowTextLengthW, IDOK, MB_ICONWARNING, MB_OK,
    MessageBoxW, SetWindowTextW, WM_COMMAND,
};
use windows::core::{HSTRING, Interface, PCWSTR, Ref, implement, w};

use crate::strings::{self, id as string_id};
use crate::utils::{ShellItemEx, WinRes, create_item_in, window_text};

/// Suffix of a Windows shortcut file.
const LINK_SUFFIX: &[u16; 4] = &[b'.' as u16, b'l' as u16, b'n' as u16, b'k' as u16];

/// Characters the editbox uses to quote/separate multiple names.
const QUOTE: u16 = b'"' as u16;
const SPACE: u16 = b' ' as u16;

// Subclass IDs only need to be unique per window.
const EDITBOX_SUBCLASS: usize = 1294;
const OK_BUTTON_SUBCLASS: usize = 5417;

#[implement(IFileDialogEvents)]
pub struct FileAndFolderEvents {
    /// The items the user confirmed, filled by `OnFileOk`.
    selection: RefCell<Vec<IShellItem>>,
    /// The folder the dialog is currently showing.
    folder: RefCell<IShellItem>,
    /// The dialog itself; needed by the window-procedure callbacks.
    dialog: IFileDialog,

    /// The last item reported by `OnSelectionChange` (used to skip duplicated events).
    last_active: RefCell<Option<IShellItem>>,
    /// The items selected at the last `OnSelectionChange`.
    last_selected: RefCell<Vec<IShellItem>>,
    /// The dialog's file-name editbox (the `ComboBoxEx32`).
    editbox: Cell<Option<HWND>>,
    /// The dialog's top-level window.
    window: Cell<Option<HWND>>,

    /// True while we are rewriting the editbox ourselves (see [`track_editbox`]).
    was_my_change: Cell<bool>,
    /// Set by [`substitute_ok`] when the OK button was clicked.
    was_ok_clicked: Cell<bool>,
    /// True once the one-time setup has run.
    is_initialized: Cell<bool>,

    // Localized labels, loaded lazily in `ensure_initialized`.
    ok_text: OnceLock<HSTRING>,             // "OK"
    open_text: OnceLock<HSTRING>,           // "Open"
    select_folder_text: OnceLock<HSTRING>,  // "Select Folder"
    path_not_found_text: OnceLock<HSTRING>, // "Path not found"
}

impl FileAndFolderEvents {
    /// Creates a handler for `dialog`, which must currently be showing `folder`.
    pub fn new(folder: IShellItem, dialog: &IFileDialog) -> Self {
        Self {
            selection: RefCell::new(Vec::new()),
            folder: RefCell::new(folder),
            dialog: dialog.clone(), // TODO: is it ok?
            last_active: RefCell::default(),
            last_selected: RefCell::default(),
            editbox: Cell::default(),
            window: Cell::default(),
            was_my_change: Cell::new(false),
            was_ok_clicked: Cell::new(false),
            is_initialized: Cell::new(false),
            ok_text: OnceLock::new(),
            open_text: OnceLock::new(),
            select_folder_text: OnceLock::new(),
            path_not_found_text: OnceLock::new(),
        }
    }

    /// The items the user confirmed (call after the dialog closed).
    pub(crate) fn selection(&self) -> Vec<IShellItem> {
        self.selection.borrow().clone()
    }

    /// The folder the dialog was showing when it closed.
    pub(crate) fn folder(&self) -> IShellItem {
        self.folder.borrow().clone()
    }

    /// Loads the localized labels, finds the dialog window and installs the window
    /// subclasses. Idempotent; returns the `(editbox, dialog window)` handles.
    fn ensure_initialized(&self) -> WinRes<(HWND, HWND)> {
        if !self.is_initialized.get() {
            let window = unsafe { shell_browser(&self.dialog)?.GetWindow()? };
            self.window.set(Some(window));

            self.ok_text
                .set(strings::load(w!("user32.dll"), string_id::OK)?)
                .ok();
            self.open_text
                .set(strings::load(w!("comdlg32.dll"), string_id::OPEN)?)
                .ok();
            self.select_folder_text
                .set(strings::load(w!("comdlg32.dll"), string_id::SELECT_FOLDER)?)
                .ok();
            self.path_not_found_text
                .set(strings::load(
                    w!("comdlg32.dll"),
                    string_id::PATH_NOT_FOUND,
                )?)
                .ok();
            unsafe {
                self.dialog.SetOkButtonLabel(self.ok_text.get().expect("label just loaded"))
            }?;

            // Track clicks on the OK button (see the module docs).
            let this = self as *const FileAndFolderEvents as usize;
            unsafe { SetWindowSubclass(window, Some(substitute_ok), OK_BUTTON_SUBCLASS, this) }
                .ok()?;

            // Stop the dialog from resetting the selection when we rewrite the editbox.
            let combo_ex = unsafe { FindWindowExW(Some(window), None, w!("ComboBoxEx32"), None) }?;
            let combo = unsafe { FindWindowExW(Some(combo_ex), None, w!("ComboBox"), None) }?;
            unsafe { SetWindowSubclass(combo, Some(track_editbox), EDITBOX_SUBCLASS, this) }
                .ok()?;
            self.editbox.set(Some(combo_ex));

            self.is_initialized.set(true);
        }

        let editbox = self.editbox.get().expect("editbox installed");
        let window = self.window.get().expect("window installed");
        Ok((editbox, window))
    }

    /// Turns a name typed into the editbox into a shell item, following `.lnk` shortcuts.
    ///
    /// Shows a localized "Path not found" warning and returns the error if the item cannot
    /// be resolved inside `folder`.
    fn resolve_name(
        &self,
        folder: &IShellItem,
        names_typed: *const u16,
        window: HWND,
        name: &HSTRING,
    ) -> WinRes<IShellItem> {
        let item = match create_item_in(folder, name) {
            Ok(item) => item,
            Err(err) => {
                if err.code() == ERROR_PATH_NOT_FOUND.to_hresult() {
                    self.show_path_not_found(window, names_typed);
                }
                return Err(err);
            }
        };

        // Resolve shortcuts to their targets, as the native dialog does.
        if name.ends_with(LINK_SUFFIX) {
            item.link_target()
        } else {
            Ok(item)
        }
    }

    /// Shows the localized "Path not found" warning for `names_typed`.
    fn show_path_not_found(&self, window: HWND, names_typed: *const u16) {
        let template = self.path_not_found_text.get().expect("localized template");
        let Ok(message) = strings::format(template, &[names_typed]) else {
            return;
        };
        let title = window_text(window);
        unsafe { MessageBoxW(Some(window), &message, &title, MB_ICONWARNING | MB_OK) };
    }
}

/// The dialog implements `IServiceProvider`; from there the shell browser of the inner
/// file list view can be reached.
fn shell_browser(dialog: &IFileDialog) -> WinRes<IShellBrowser> {
    let provider = dialog.cast::<IServiceProvider>()?;
    unsafe { provider.QueryService::<IShellBrowser>(&SID_STopLevelBrowser) }
}

/// Whether the high bit of a key's state is set, i.e. the key is currently down.
fn is_key_pressed(key: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(key.0 as i32) as u16 & 0x8000 != 0 }
}

/// Swallows the `EN_CHANGE` notifications generated while we rewrite the editbox.
///
/// Everything else is forwarded unchanged, keeping the OK button label in sync with what
/// the user types.
extern "system" fn track_editbox(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    if message == WM_COMMAND && EN_CHANGE == ((wparam.0 >> 16) & 0xFFFF) as u32 {
        let events = unsafe { &*(ref_data as *const FileAndFolderEvents) };
        if events.was_my_change.get() {
            // Our own edit, generated in `OnSelectionChange`. Do not forward it to the
            // dialog, otherwise the selection would be reset.
            return LRESULT(0);
        }

        let dialog = &events.dialog;
        let empty = events
            .editbox
            .get()
            .is_some_and(|editbox| unsafe { GetWindowTextLengthW(editbox) } == 0);
        let label = if empty {
            events.select_folder_text.get().expect("localized label")
        } else {
            events.ok_text.get().expect("localized label")
        };
        let _ = unsafe { dialog.SetOkButtonLabel(label) };
    }

    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// Intercepts OK clicks: instead of letting the dialog close with a files-only selection,
/// force a folder change (which `OnFolderChanging` turns into the real selection handling).
extern "system" fn substitute_ok(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    if message == WM_COMMAND
        && BN_CLICKED == ((wparam.0 >> 16) & 0xFFFF) as u32
        && IDOK.0 == (wparam.0 & 0xFFFF) as i32
    {
        let events = unsafe { &*(ref_data as *const FileAndFolderEvents) };
        let dialog = &events.dialog;
        // Ask the dialog to (re)apply the current folder. The resulting
        // `OnFolderChanging` notices `was_ok_clicked` and runs the real `OnFileOk`.
        let folder_was_set = unsafe {
            dialog
                .GetFolder()
                .and_then(|folder| dialog.SetFolder(&folder))
        }
            .is_ok();
        if folder_was_set {
            events.was_ok_clicked.set(true);
        }
        // Swallow the click: the dialog must not close on its own.
        return LRESULT(0);
    }

    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[allow(non_snake_case)]
impl IFileDialogEvents_Impl for FileAndFolderEvents_Impl {
    /// Reads the names from the editbox and turns them into shell items.
    fn OnFileOk(&self, pfd: Ref<'_, IFileDialog>) -> WinRes<()> {
        let dialog = pfd.ok()?;
        let (editbox, window) = self.ensure_initialized()?;
        let folder = unsafe { dialog.GetFolder()? };

        let names_typed = window_text(editbox);
        let mut selection = Vec::new();

        if names_typed.is_empty() {
            selection.push(folder);
        } else if names_typed.contains(&QUOTE) {
            // Multiple quoted names: "a.txt" "b.pdf"
            for name in names_typed
                .split(|&c| c == QUOTE)
                .filter(|part| part.iter().any(|&c| c != SPACE))
            {
                selection.push(self.resolve_name(
                    &folder,
                    names_typed.as_ptr(),
                    window,
                    &HSTRING::from_wide(name),
                )?);
            }
        } else {
            selection.push(self.resolve_name(
                &folder,
                names_typed.as_ptr(),
                window,
                &names_typed,
            )?);
        }

        *self.selection.borrow_mut() = selection;
        unsafe { dialog.Close(S_OK) }
    }

    /// Rejects the folder change and ends the dialog when the OK button was clicked or
    /// Shift+Enter was pressed; otherwise just clears the editbox.
    fn OnFolderChanging(
        &self,
        pfd: Ref<'_, IFileDialog>,
        psifolder: Ref<'_, IShellItem>,
    ) -> WinRes<()> {
        // It is odd, but the standard dialog updates the "last folder" even for the
        // folder the user selected, so we do the same.
        *self.folder.borrow_mut() = psifolder.ok()?.to_owned();

        let ok_was_clicked = self.was_ok_clicked.get();
        let shift_enter = is_key_pressed(VK_SHIFT) && is_key_pressed(VK_RETURN);
        if ok_was_clicked || shift_enter {
            self.was_ok_clicked.set(false);
            return self.OnFileOk(pfd);
        }

        if let Some(editbox) = self.editbox.get() {
            unsafe { SetWindowTextW(editbox, w!(""))? };
        }
        Ok(())
    }

    fn OnFolderChange(&self, _pfd: Ref<'_, IFileDialog>) -> WinRes<()> {
        Ok(())
    }

    /// Mirrors the current selection (files *and* folders) into the editbox.
    fn OnSelectionChange(&self, pfd: Ref<'_, IFileDialog>) -> WinRes<()> {
        let dialog = pfd.ok()?;

        let browser = shell_browser(dialog)?;
        let view = unsafe { browser.QueryActiveShellView()? };
        let (editbox, _window) = self.ensure_initialized()?;

        // This event fires twice per user action; skip the second one.
        let active = unsafe { dialog.GetCurrentSelection()? };
        if self
            .last_active
            .borrow()
            .as_ref()
            .is_some_and(|last| *last == active)
        {
            return Ok(());
        }
        *self.last_active.borrow_mut() = Some(active);

        // The native dialog only reports selected *files*; ask the shell view directly so
        // that folders are included as well.
        let Ok(array) = (unsafe { view.GetItemObject::<IShellItemArray>(SVGIO_SELECTION) }) else {
            self.last_selected.borrow_mut().clear();
            return Ok(());
        };
        let count = unsafe { array.GetCount()? };
        let mut items = Vec::with_capacity(count as usize);
        for index in 0..count {
            items.push(unsafe { array.GetItemAt(index as _)? });
        }
        *self.last_selected.borrow_mut() = items;

        let last_selected = self.last_selected.borrow();
        let names: Vec<HSTRING> = last_selected
            .iter()
            .map(IShellItem::relative_editing_name)
            .collect();
        let names_inlined: HSTRING = match names.as_slice() {
            [single] => single.clone(),
            many => many
                .iter()
                .map(|name| format!("\"{name}\" "))
                .collect::<String>()
                .into(),
        };

        let all_files = last_selected
            .iter()
            .all(|item| item.is_file().unwrap_or(false));
        let all_folders = last_selected
            .iter()
            .all(|item| item.is_directory().unwrap_or(false));
        let label = if all_files {
            self.open_text.get().expect("localized label")
        } else if all_folders {
            self.select_folder_text.get().expect("localized label")
        } else {
            self.ok_text.get().expect("localized label")
        };
        unsafe { dialog.SetOkButtonLabel(label) }?;

        // Mirror the selection into the editbox, suppressing the EN_CHANGE notifications
        // while we do so.
        self.was_my_change.set(true);
        let result = unsafe { SetWindowTextW(editbox, PCWSTR(names_inlined.as_ptr())) };
        self.was_my_change.set(false);
        result?;

        Ok(())
    }

    fn OnShareViolation(
        &self,
        _pfd: Ref<'_, IFileDialog>,
        _psi: Ref<'_, IShellItem>,
    ) -> WinRes<FDE_SHAREVIOLATION_RESPONSE> {
        Ok(FDESVR_DEFAULT)
    }

    fn OnTypeChange(&self, _pfd: Ref<'_, IFileDialog>) -> WinRes<()> {
        Ok(())
    }

    fn OnOverwrite(
        &self,
        _pfd: Ref<'_, IFileDialog>,
        _psi: Ref<'_, IShellItem>,
    ) -> WinRes<FDE_OVERWRITE_RESPONSE> {
        Ok(FDEOR_DEFAULT)
    }
}