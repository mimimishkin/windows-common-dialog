use windows::core::{HSTRING, PCWSTR};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;

#[derive(Debug)]
pub struct WinFileFilter {
    pub name: HSTRING,
    pub patterns: HSTRING,
}

impl WinFileFilter {
    pub fn to_comdlg_filterspec(&self) -> COMDLG_FILTERSPEC {
        COMDLG_FILTERSPEC {
            pszName: PCWSTR(self.name.as_ptr()),
            pszSpec: PCWSTR(self.patterns.as_ptr()),
        }
    }
}