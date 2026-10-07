//! Filesystem paths of some well-known folders.

use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Pictures,
    FOLDERID_Videos,
};

use crate::utils::{WinRes, known_folder_path};

/// Loads the paths of the desktop, documents, downloads, music, pictures and videos folders.
pub fn load_folders() -> WinRes<Vec<String>> {
    [
        FOLDERID_Desktop,
        FOLDERID_Documents,
        FOLDERID_Downloads,
        FOLDERID_Music,
        FOLDERID_Pictures,
        FOLDERID_Videos,
    ]
        .iter()
        .map(known_folder_path)
        .collect()
}
