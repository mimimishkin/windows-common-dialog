use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub struct FileFilter(pub String);

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

#[derive(Debug, Clone, Default)]
pub enum ChooserDialogOwner {
    #[default] 
    None,
    HWND { ptr: *mut core::ffi::c_void },
    NSWindow { ptr: *const core::ffi::c_void },
}

unsafe impl Sync for ChooserDialogOwner {}
unsafe impl Send for ChooserDialogOwner {}

#[derive(Debug, Clone)]
pub struct ChooserDialogParams<'a> {
    pub id: Option<u128>,
    pub title: Option<&'a str>,
    pub filters: Vec<FileFilter>,
    pub mode: ChoosingMode,
    pub multiple: bool,
    pub initial_directory: Option<&'a str>,
    pub suggested_name: Option<&'a str>,
    pub owner: ChooserDialogOwner,
}

#[derive(Debug, Clone)]
pub struct ChooserDialogResult {
    pub selected: Vec<String>,
    pub last_folder: Option<String>,
}

impl ChooserDialogResult {
    pub fn new(selected: Vec<String>, last_folder: Option<String>) -> Self {
        Self { selected, last_folder }
    }
}

#[derive(Debug, Clone)]
pub struct ChooserDialogError(pub String);

impl Display for ChooserDialogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in chooser dialog: {}", self.0)
    }
}

type ChooserRes<T> = Result<T, ChooserDialogError>;



#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
pub use win::*;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
pub use mac::*;


#[cfg(any(target_os = "windows", target_os = "macos"))]
#[cfg(test)]
mod tests {
    use crate::dialog::{choose, ChooserDialogOwner, ChooserDialogParams, ChoosingMode, FileFilter};
    use std::sync::Arc;

    #[test]
    fn test_open_file_save_dialog() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Save File"),
            filters: vec![
                FileFilter("Text Files\0*.txt".into()),
                FileFilter("All Files\0*.*".into())
            ],
            mode: ChoosingMode::Saving,
            multiple: false,
            initial_directory: None,
            suggested_name: Some("document.txt"),
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_file_chooser() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Select a File"),
            filters: vec![
                FileFilter("All Files\0*.*".into()),
                FileFilter("Documents\0*.txt\0*.docx\0*.doc".into())
            ],
            mode: ChoosingMode::FilesOnly,
            multiple: false,
            initial_directory: None,
            suggested_name: None,
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_directory_chooser() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Select a Directory"),
            filters: vec![],
            mode: ChoosingMode::DirectoriesOnly,
            multiple: true,
            initial_directory: None,
            suggested_name: None,
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }

    #[test]
    fn test_open_file_or_directory_chooser() {
        let params = ChooserDialogParams {
            id: None,
            title: Some("Select a Directory"),
            filters: vec![
                FileFilter("All Files\0*.*".into()),
                FileFilter("Documents\0*.txt\0*.docx\0*.doc".into())
            ],
            mode: ChoosingMode::FilesAndDirectories,
            multiple: true,
            initial_directory: None,
            suggested_name: None,
            owner: ChooserDialogOwner::None,
        };

        choose(Arc::new(params), |res| {
            println!("{:#?}", res);
        });
    }
}