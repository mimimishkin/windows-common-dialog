mod utils;
mod params;
mod file_filter;
mod dialog;
mod mime;
mod folders;

use crate::win::utils::{find_window_with, ExtractName};
use crate::{ChooserDialogOwner, ChooserDialogParams as RawChooserDialogParams, ChooserDialogResult, ChooserRes, ChoosingMode};
use file_filter::*;
use params::*;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::Arc;
use utils::{with_com, WinRes};
use windows::core::HSTRING;
use windows::Win32::Foundation::HWND;
use ChoosingMode::Saving;

pub fn choose(params: Arc<RawChooserDialogParams>, callback: impl FnOnce(ChooserRes<ChooserDialogResult>) + Send + 'static) {
    let params = WinChooserDialogParams {
        guid: params.id.map(|id| id.into()),
        title: params.title.map(HSTRING::from),
        filters: params.filters.iter().map(|f| {
            let (name, patterns) = f.split();
            WinFileFilter {
                name: name.unwrap_or_default().into(),
                patterns: patterns.replace('\0', ";").into(),
            }
        }).collect(),
        default_extension: if let Saving = params.mode {
            // find the first non * extension
            params.filters.iter().find_map(|f| {
                let pattern = f.elements().next().unwrap();
                let ext = pattern.split_once('.').expect("No extension given").1;
                (ext != "*").then(|| ext.into())
            })
        } else {
            None
        },
        mode: params.mode,
        multiple: params.multiple,
        initial_directory: params.initial_directory.map(HSTRING::from),
        suggested_name: params.suggested_name.map(HSTRING::from),
        owner: match params.owner {
            ChooserDialogOwner::HWND { ptr } => Some(HWND(ptr)),
            ChooserDialogOwner::Desc { title, x, y, width, height } => unsafe {
                let title = (!title.is_null()).then(|| CStr::from_ptr(title));
                let title = title.and_then(|title| Some(HSTRING::from(title.to_str().ok()?)));
                find_window_with(title.as_ref(), x, y, width, height).ok()
            }
            _ => None
        }
    };
    let params = Arc::new(params);

    let result: WinRes<ChooserDialogResult> = with_com(|| {
        let (selection, last_folder) = dialog::choose(&params)?;

        Ok(ChooserDialogResult::new(
            selection.iter().map(|i| i.absolute_parsing_name().to_string()).collect(),
            last_folder.map(|i| i.absolute_parsing_name().to_string())
        ))
    });

    callback(result.map_err(|e| e.into()));
}

pub fn load_mime_table() -> ChooserRes<HashMap<String, Vec<String>>> {
    Ok(mime::load_extensions()?)
}

pub fn load_folders() -> ChooserRes<Vec<String>> {
    Ok(folders::load_folders()?)
}