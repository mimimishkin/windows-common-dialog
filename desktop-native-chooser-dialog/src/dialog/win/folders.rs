use crate::dialog::win::utils::{known_folder_path, WinRes};
use windows::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Pictures, FOLDERID_Videos};

pub fn load_folders() -> WinRes<Vec<String>> {
    let folders = [FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Pictures, FOLDERID_Videos];
    folders.iter().map(known_folder_path).collect()
}