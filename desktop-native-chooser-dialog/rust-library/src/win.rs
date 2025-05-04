mod utils;
mod params;
mod file_filter;
mod dialog;
mod mime;
mod folders;

use crate::{ChooserDialogOwner, ChooserDialogParams as RawChooserDialogParams, ChooserDialogResult, ChooserRes, ChoosingMode};
use utils::{with_com, WinRes};
use dialog::*;
use file_filter::*;
use params::*;
use std::collections::HashMap;
use windows::core::HSTRING;
use windows::Win32::Foundation::HWND;
use ChoosingMode::Saving;
use crate::win::utils::{find_window_with, ExtractName};

pub fn choose_impl(params: &RawChooserDialogParams, callback: impl FnOnce(ChooserRes<ChooserDialogResult>)) {
    let result: WinRes<ChooserDialogResult> = with_com(|| {
        let params = WinChooserDialogParams {
            guid: params.id.map(|id| id.into()),
            title: params.title.as_ref().map(HSTRING::from),
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
            initial_directory: params.initial_directory.as_ref().map(HSTRING::from),
            suggested_name: params.suggested_name.as_ref().map(HSTRING::from),
            owner: match params.owner {
                ChooserDialogOwner::HWND { ptr_addr } => Some(HWND(ptr_addr as _)),
                ChooserDialogOwner::Desc { ref title, x, y, width, height } => {
                    let title = title.as_ref().map(HSTRING::from);
                    find_window_with(title.as_ref(), x, y, width, height).ok()
                }
                _ => None
            }
        };

        let (selection, last_folder) = choose(&params)?;

        Ok(ChooserDialogResult::new(
            selection.iter().map(|i| i.absolute_parsing_name().to_string()).collect(),
            last_folder.map(|i| i.absolute_parsing_name().to_string())
        ))
    });
    
    // TODO: Async
    callback(result.map_err(|e| e.into()));
}

pub fn load_extensions_impl() -> ChooserRes<HashMap<String, Vec<String>>> {
    Ok(mime::load_extensions()?)
}

pub fn load_folders_impl() -> ChooserRes<Vec<String>> {
    Ok(folders::load_folders()?)
}