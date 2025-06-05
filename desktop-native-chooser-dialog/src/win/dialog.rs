use crate::win::file_filter::WinFileFilter;
use crate::win::fnf_events::FileAndFolderEvents;
use crate::win::params::ChoosingMode::{DirectoriesOnly, FilesAndDirectories, Saving};
use crate::win::params::WinChooserDialogParams;
use crate::win::utils::{create_item, load_string, WinRes, HRESULT_CANCELLED};
use windows::core::{w, Interface, HSTRING};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{FileOpenDialog, FileSaveDialog, IFileDialog, IFileDialogEvents, IFileOpenDialog, IFileSaveDialog, IShellItem, FOS_ALLOWMULTISELECT, FOS_PICKFOLDERS, FOS_SUPPORTSTREAMABLEITEMS};

pub fn choose(params: &WinChooserDialogParams) -> WinRes<(Vec<IShellItem>, Option<IShellItem>)> { unsafe {
    let dialog = if params.mode != Saving {
        let res: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        res.cast::<IFileDialog>()?
    } else {
        let res: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
        res.cast::<IFileDialog>()?
    };

    // tell the dialog not to download files
    dialog.SetOptions(dialog.GetOptions()? | FOS_SUPPORTSTREAMABLEITEMS)?;

    if let Some(guid) = params.id {
        dialog.SetClientGuid(&guid)?;
    }

    if let Some(title) = &params.title {
        dialog.SetTitle(title)?;
    }

    if !params.filters.is_empty() {
        let types = params.filters.iter().map(WinFileFilter::to_comdlg_filterspec).collect::<Vec<_>>();
        dialog.SetFileTypes(&types)?;
    }

    if let Some(name) = &params.suggested_name {
        dialog.SetFileName(name)?;
    }

    if let Some(dir) = &params.initial_directory {
        let item = create_item(dir)?;
        dialog.SetFolder(&item)?;
    }

    let handle_result = |res: WinRes<()>| -> WinRes<Option<Vec<IShellItem>>> {
        match res {
            Ok(_) => Ok(None),
            Err(err) => {
                if let HRESULT_CANCELLED = err.code() {
                    Ok(Some(vec![]))
                } else {
                    Err(err)
                }
            },
        }
    };

    let selected: Vec<IShellItem> = if let Saving = params.mode {
        let dialog: IFileSaveDialog = dialog.cast()?;

        if let Some(res) = handle_result(dialog.Show(params.owner))? {
            res
        } else {
            vec![dialog.GetResult()?]
        }
    } else {
        let dialog: IFileOpenDialog = dialog.cast()?;

        if params.multiple {
            let new_options = dialog.GetOptions()? | FOS_ALLOWMULTISELECT;
            dialog.SetOptions(new_options)?;
        }

        if params.mode != FilesAndDirectories {
            if params.mode == DirectoriesOnly {
                let new_options = dialog.GetOptions()? | FOS_PICKFOLDERS;
                dialog.SetOptions(new_options)?;
            }

            if let Some(res) = handle_result(dialog.Show(params.owner))? {
                res
            } else {
                let res = &dialog.GetResults()?;
                let count = res.GetCount()?;
                (0..count).map(|i| res.GetItemAt(i)).collect::<WinRes<_>>()?
            }
        } else {
            if params.title.is_none() {
                let mut title_buffer = [0u16; 128];
                let module = GetModuleHandleW(w!("comdlg32.dll"))?;
                let open_file_len = load_string(module, 436 /* Open File */, &mut title_buffer)?;
                
                title_buffer[open_file_len    ] = ' ' as u16;
                title_buffer[open_file_len + 1] = '/' as u16;
                title_buffer[open_file_len + 2] = ' ' as u16;
                
                let select_folder_len = load_string(module, 439 /* Select Folder */, &mut title_buffer[(open_file_len + 3)..])?;
                let title = HSTRING::from_wide(&title_buffer[..(open_file_len + 3 + select_folder_len)]);
                
                dialog.SetTitle(&title)?;
            }

            let mut user_selection: Vec<IShellItem> = vec![];
            let mut last_folder: IShellItem = dialog.GetFolder()?;
            let events = FileAndFolderEvents::new(&mut user_selection, &mut last_folder, &dialog);
            let events: IFileDialogEvents = events.into();
            let cookie = dialog.Advise(&events)?;

            let res = handle_result(dialog.Show(params.owner));
            dialog.Unadvise(cookie)?;
            let selection = res?.unwrap_or(user_selection);

            return Ok((selection, Some(last_folder)))
        }
    };

    let folder = dialog.GetFolder();
    Ok((selected, folder.ok()))
} }
