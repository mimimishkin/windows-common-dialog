//! Running the common file dialogs.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_PICKFOLDERS, FOS_SUPPORTSTREAMABLEITEMS, FileOpenDialog,
    FileSaveDialog, IFileDialog, IFileDialogEvents, IFileOpenDialog, IFileSaveDialog, IShellItem,
};
use windows::core::{ComObject, HSTRING, Interface, w};

use crate::file_filter::WinFileFilter;
use crate::fnf_events::FileAndFolderEvents;
use crate::params::ChoosingMode::{self, DirectoriesOnly, FilesAndDirectories, Saving};
use crate::params::WinChooserDialogParams;
use crate::strings::{self, id as string_id};
use crate::utils::{HRESULT_CANCELLED, WinRes, create_item, with_com};

/// Shows a dialog configured by `params` and returns what the user picked.
///
/// The first element is the selected items (empty when the user canceled); the second is
/// the folder the dialog was showing, if it could be determined.
pub fn choose(params: &WinChooserDialogParams) -> WinRes<(Vec<IShellItem>, Option<IShellItem>)> {
    with_com(|| choose_impl(params))
}

fn choose_impl(params: &WinChooserDialogParams) -> WinRes<(Vec<IShellItem>, Option<IShellItem>)> {
    let dialog = create_dialog(params.mode)?;
    configure(&dialog, params)?;

    if params.mode == Saving {
        let dialog: IFileSaveDialog = dialog.cast()?;
        return save(&dialog, params);
    }

    let dialog: IFileOpenDialog = dialog.cast()?;
    configure_open(&dialog, params)?;

    if params.mode == FilesAndDirectories {
        pick_files_and_folders(&dialog, params)
    } else {
        pick_items(&dialog, params)
    }
}

/// Creates the native dialog for `mode`.
fn create_dialog(mode: ChoosingMode) -> WinRes<IFileDialog> {
    unsafe {
        if mode == Saving {
            let dialog: IFileSaveDialog =
                CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.cast()
        } else {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.cast()
        }
    }
}

/// Applies the parameters shared by every dialog mode.
fn configure(dialog: &IFileDialog, params: &WinChooserDialogParams) -> WinRes<()> {
    unsafe {
        // Tell the dialog not to download files eagerly (cloud placeholders stay streamable).
        dialog.SetOptions(dialog.GetOptions()? | FOS_SUPPORTSTREAMABLEITEMS)?;

        if let Some(guid) = params.id {
            dialog.SetClientGuid(&guid)?;
        }

        if let Some(title) = &params.title {
            dialog.SetTitle(title)?;
        }

        if !params.filters.is_empty() {
            let types: Vec<_> = params
                .filters
                .iter()
                .map(WinFileFilter::as_comdlg_filterspec)
                .collect();
            dialog.SetFileTypes(&types)?;
        }

        if let Some(name) = &params.suggested_name {
            dialog.SetFileName(name)?;
        }

        if let Some(dir) = &params.initial_directory {
            dialog.SetFolder(&create_item(dir)?)?;
        }

        Ok(())
    }
}

/// Applies the open-dialog options: multi selection and/or folder picking.
fn configure_open(dialog: &IFileOpenDialog, params: &WinChooserDialogParams) -> WinRes<()> {
    let mut options = unsafe { dialog.GetOptions()? };
    if params.multiple {
        options |= FOS_ALLOWMULTISELECT;
    }
    if params.mode == DirectoriesOnly {
        options |= FOS_PICKFOLDERS;
    }
    unsafe { dialog.SetOptions(options) }?;
    Ok(())
}

/// Shows `dialog`, returning `Ok(true)` when it closed normally and `Ok(false)` when the
/// user canceled.
fn run(dialog: &IFileDialog, owner: Option<HWND>) -> WinRes<bool> {
    match unsafe { dialog.Show(owner) } {
        Ok(()) => Ok(true),
        Err(err) if err.code() == HRESULT_CANCELLED => Ok(false),
        Err(err) => Err(err),
    }
}

fn save(
    dialog: &IFileSaveDialog,
    params: &WinChooserDialogParams,
) -> WinRes<(Vec<IShellItem>, Option<IShellItem>)> {
    let selected = if run(dialog, params.owner)? {
        vec![unsafe { dialog.GetResult()? }]
    } else {
        Vec::new()
    };
    Ok((selected, unsafe { dialog.GetFolder() }.ok()))
}

fn pick_items(
    dialog: &IFileOpenDialog,
    params: &WinChooserDialogParams,
) -> WinRes<(Vec<IShellItem>, Option<IShellItem>)> {
    let selected = if run(dialog, params.owner)? {
        collect_results(dialog)?
    } else {
        Vec::new()
    };
    Ok((selected, unsafe { dialog.GetFolder() }.ok()))
}

/// Collects every selected item from `dialog`.
fn collect_results(dialog: &IFileOpenDialog) -> WinRes<Vec<IShellItem>> {
    let results = unsafe { dialog.GetResults()? };
    (0..unsafe { results.GetCount()? })
        .map(|index| unsafe { results.GetItemAt(index) })
        .collect()
}

/// Runs the dialog with the file/folder hybrid handler installed.
fn pick_files_and_folders(
    dialog: &IFileOpenDialog,
    params: &WinChooserDialogParams,
) -> WinRes<(Vec<IShellItem>, Option<IShellItem>)> {
    if params.title.is_none() {
        unsafe { dialog.SetTitle(&mixed_title()?) }?;
    }

    let folder = unsafe { dialog.GetFolder()? };
    let events = ComObject::new(FileAndFolderEvents::new(folder, dialog));
    let sink: IFileDialogEvents = events.to_interface();
    let cookie = unsafe { dialog.Advise(&sink)? };

    let was_shown = run(dialog, params.owner);
    unsafe { dialog.Unadvise(cookie) }?;

    let selected = if was_shown? {
        events.get().selection()
    } else {
        Vec::new()
    };
    Ok((selected, Some(events.get().folder())))
}

/// The default title of the hybrid dialog: "Open File / Select Folder".
fn mixed_title() -> WinRes<HSTRING> {
    let open_file = strings::load(w!("comdlg32.dll"), string_id::OPEN_FILE)?;
    let select_folder = strings::load(w!("comdlg32.dll"), string_id::SELECT_FOLDER)?;
    Ok(HSTRING::from(format!("{open_file} / {select_folder}")))
}
