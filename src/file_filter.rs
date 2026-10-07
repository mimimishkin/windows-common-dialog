//! File type filters offered by the dialog.

use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::core::{HSTRING, PCWSTR};

/// A named file type filter, e.g. `("Text files", "*.txt;*.md")`.
///
/// `patterns` is a semicolon-separated list of wildcard patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinFileFilter {
    /// Human-readable name shown in the type dropdown.
    pub name: HSTRING,
    /// Wildcard patterns, e.g. `"*.txt;*.md"`.
    pub patterns: HSTRING,
}

impl WinFileFilter {
    /// Creates a new filter.
    pub fn new(name: impl Into<HSTRING>, patterns: impl Into<HSTRING>) -> Self {
        Self {
            name: name.into(),
            patterns: patterns.into(),
        }
    }

    /// Returns the matching `COMDLG_FILTERSPEC`.
    pub fn as_comdlg_filterspec(&self) -> COMDLG_FILTERSPEC {
        COMDLG_FILTERSPEC {
            pszName: PCWSTR(self.name.as_ptr()),
            pszSpec: PCWSTR(self.patterns.as_ptr()),
        }
    }
}
