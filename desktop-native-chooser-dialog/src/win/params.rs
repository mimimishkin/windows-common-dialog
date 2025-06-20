use crate::win::file_filter::WinFileFilter;
use windows::core::{GUID, HSTRING};
use windows::Win32::Foundation::HWND;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ChoosingMode {
    Saving = 0,
    FilesOnly = 1,
    DirectoriesOnly = 2,
    FilesAndDirectories = 3,
}

#[derive(Debug)]
pub struct WinChooserDialogParams {
    pub id: Option<GUID>,
    pub title: Option<HSTRING>,
    pub filters: Vec<WinFileFilter>,
    pub mode: ChoosingMode,
    pub multiple: bool,
    pub initial_directory: Option<HSTRING>,
    pub suggested_name: Option<HSTRING>,
    pub owner: Option<HWND>,
}

unsafe impl Send for WinChooserDialogParams {}
unsafe impl Sync for WinChooserDialogParams {}