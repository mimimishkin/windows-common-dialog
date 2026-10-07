//! Parameters describing what a dialog should look like and accept.

use windows::Win32::Foundation::HWND;
use windows::core::{GUID, HSTRING};

use crate::file_filter::WinFileFilter;

/// What the dialog is used for.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ChoosingMode {
    /// The dialog saves a single file.
    Saving = 0,
    /// Pick one or more files.
    #[default]
    FilesOnly = 1,
    /// Pick one or more folders.
    DirectoriesOnly = 2,
    /// Pick files and folders at the same time.
    FilesAndDirectories = 3,
}

/// Everything needed to configure and run a dialog. See [`crate::dialog::choose`].
#[derive(Debug, Default)]
pub struct WinChooserDialogParams {
    /// Client GUID used to persist the dialog's state between sessions.
    pub id: Option<GUID>,
    /// Dialog title. When `None`, the system provides a default one.
    pub title: Option<HSTRING>,
    /// File type filters offered in the type dropdown. Empty means "all files".
    pub filters: Vec<WinFileFilter>,
    /// What the dialog is used for.
    pub mode: ChoosingMode,
    /// Allow picking several items at once (open dialogs only).
    pub multiple: bool,
    /// The folder the dialog opens in.
    pub initial_directory: Option<HSTRING>,
    /// The initially suggested file name.
    pub suggested_name: Option<HSTRING>,
    /// Optional owner window for the dialog.
    pub owner: Option<HWND>,
}