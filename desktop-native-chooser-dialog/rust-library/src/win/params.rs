use crate::win::WinFileFilter;
use crate::ChoosingMode;
use windows::Win32::Foundation::HWND;
use windows::core::{GUID, HSTRING};

pub struct WinChooserDialogParams {
    pub guid: Option<GUID>,
    pub title: Option<HSTRING>,
    pub filters: Vec<WinFileFilter>,
    pub default_extension: Option<HSTRING>,
    pub mode: ChoosingMode,
    pub multiple: bool,
    pub initial_directory: Option<HSTRING>,
    pub suggested_name: Option<HSTRING>,
    pub owner: Option<HWND>,
}