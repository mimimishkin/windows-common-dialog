//! This module provides a cross-platform file chooser dialog interface.
//! It shouldn't be used directly, it's only for outer kotlin project.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub mod desktop_native_chooser_dialog {
    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ChoosingMode {
    Saving,
    FilesOnly,
    FilesAndDirectories,
    DirectoriesOnly,
}


/// Inlined file filter. 
/// Filter name and elements are separated by '\0'.
/// For example, `Documents (*.txt, *.doc, *.docx)` becomes `"Documents\0*.txt\0*.doc\0*.docx"`
#[derive(Debug, Clone)]
pub struct FileFilter(String);

uniffi::custom_newtype!(FileFilter, String);

impl FileFilter {
    pub fn split(&self) -> (Option<&str>, &str) {
        let (name, patterns) = self.0.split_once('\0').expect("Wrong filter format");
        ((!name.is_empty()).then_some(name), patterns)
    }
    
    pub fn name(&self) -> Option<&str> {
        let name = self.0.split_once('\0').expect("Wrong filter format").0;
        (!name.is_empty()).then_some(name)
    }

    pub fn elements_inlined(&self) -> &str {
        self.0.split_once('\0').expect("Wrong filter format").1
    }
    
    pub fn elements(&self) -> impl Iterator<Item=&str> {
        self.0.split('\0').skip(1)
    }
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum ChooserDialogOwner {
    HWND { ptr_addr: u64 },
    X11 { id: u64 },
    Wayland { surface_ptr_addr: u64, display_ptr_addr: u64 },
    NSView { ptr_addr: u64 },
    Desc { title: Option<String>, x: i32, y: i32, width: i32, height: i32 },
    None
}

#[derive(Debug, Clone, uniffi::Object)]
pub struct ChooserDialogParams {
    pub id: Option<u128>,
    pub title: Option<String>,
    pub filters: Vec<FileFilter>,
    pub mode: ChoosingMode,
    pub multiple: bool,
    pub initial_directory: Option<String>,
    pub suggested_name: Option<String>,
    pub owner: ChooserDialogOwner,
}

// #[uniffi::export]
impl ChooserDialogParams {
    #[uniffi::constructor]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<u128>,
        title: Option<String>,
        filters: Vec<FileFilter>,
        mode: ChoosingMode,
        multiple: bool,
        initial_directory: Option<String>,
        suggested_name: Option<String>,
        owner: ChooserDialogOwner
    ) -> Self {
        Self {
            // id: id_part1.map(|id1| ((id1 as u128) << 64) | (id_part2.unwrap() as u128)),
            id,
            title,
            filters,
            mode,
            multiple,
            initial_directory,
            suggested_name,
            owner
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ChooserDialogResult {
    selected: Vec<String>,
    last_folder: Option<String>,
}

impl ChooserDialogResult {
    pub fn new(selected: Vec<String>, last_folder: Option<String>) -> Self {
        Self { selected, last_folder }
    }
}

// TODO: custom type for result

#[derive(Debug, Clone, uniffi::Error)]
pub enum ChooserDialogError {
    Generic(String),
}

impl Display for ChooserDialogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChooserDialogError::Generic(msg) => write!(f, "ChooserDialogError: {msg}"),
        }
    }
}

type ChooserRes<T> = Result<T, ChooserDialogError>;

#[cfg(target_os = "windows")]
mod win;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod xdg;
#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "macos")]
use mac::*;
#[cfg(target_os = "windows")]
use win::*;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use xdg::*;

pub(crate) fn try_map_ext<T>(pattern: &str, process: impl Fn(&str) -> Option<T>) -> Option<T> {
    let full_i = pattern.rfind("*.");
    let mut res = full_i.and_then(|i| process(&pattern[i + 2..]));

    if res.is_none() {
        let i = pattern.rfind('.').filter(|&i| full_i.is_none() || i != full_i.unwrap() + 1);
        res = i.and_then(|i| process(&pattern[i + 1..]));
    }

    res
}

// #[uniffi::export]
pub fn choose(params: Arc<ChooserDialogParams>, callback: impl FnOnce(ChooserRes<ChooserDialogResult>) + Send + 'static) {
    choose_impl(params, callback)
}

// #[uniffi::export]
fn load_extensions() -> ChooserRes<HashMap<String, Vec<String>>> {
    load_extensions_impl()
}

// #[uniffi::export]
fn load_folders() -> ChooserRes<Vec<String>> {
    load_folders_impl()
}

// This generates extra Rust code required by UniFFI.
uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::{choose, ChooserDialogOwner, ChooserDialogParams, ChoosingMode, FileFilter};

    #[test]
    fn test_open_file_save_dialog() {
        let params = ChooserDialogParams::new(
            None,
            Some("Save File".to_string()),
            vec![
                FileFilter("Text Files\0*.txt".to_string()),
                FileFilter("All Files\0*.*".to_string())
            ],
            ChoosingMode::Saving,
            false,
            None,
            Some("document.txt".to_string()), // Suggested filename
            ChooserDialogOwner::None,
        );

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_file_chooser() {
        let params = ChooserDialogParams::new(
            None,
            Some("Select a File".to_string()),
            vec![
                FileFilter("All Files\0*.*".to_string()), 
                FileFilter("Documents\0*.txt\0*.docx\0*.doc".to_string())
            ],
            ChoosingMode::FilesOnly,
            false,
            None,
            None,
            ChooserDialogOwner::None,
        );

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_directory_chooser() {
        let params = ChooserDialogParams::new(
            None,
            Some("Select a Directory".to_string()),
            vec![],
            ChoosingMode::DirectoriesOnly,
            true,
            None,
            None,
            ChooserDialogOwner::None,
        );

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_file_or_directory_chooser() {
        let params = ChooserDialogParams::new(
            None,
            Some("Select a Directory".to_string()),
            vec![
                FileFilter("All Files\0*.*".to_string()),
                FileFilter("Documents\0*.txt\0*.docx\0*.doc".to_string())
            ],
            ChoosingMode::FilesAndDirectories,
            true,
            None,
            None,
            ChooserDialogOwner::None,
        );

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }
}