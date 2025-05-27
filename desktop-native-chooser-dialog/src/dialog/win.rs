mod utils;
mod params;
mod file_filter;
mod dialog;
mod mime;
mod folders;

use crate::dialog::win::utils::ExtractName;
use crate::dialog::ChooserRes;
use crate::dialog::ChoosingMode::Saving;
use crate::{ChooserDialogOwner, ChooserDialogParams, ChooserDialogResult};
use file_filter::*;
use jni::objects::JObject;
use jni::sys::{jobject, jobjectArray, jsize};
use jni::JNIEnv;
use params::*;
use std::sync::Arc;
use utils::with_com;
use windows::core::HSTRING;
use windows::Win32::Foundation::HWND;

pub fn choose(params: Arc<ChooserDialogParams>, callback: impl FnOnce(ChooserRes<ChooserDialogResult>) + Send + 'static) {
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
            _ => None
        }
    };
    let params = Arc::new(params);

    std::thread::spawn(move || {
        with_com(|| {
            let result = dialog::choose(&params).map(|(selection, last_folder)| {
                ChooserDialogResult::new(
                    selection.iter().map(|i| i.absolute_parsing_name().to_string()).collect(),
                    last_folder.map(|i| i.absolute_parsing_name().to_string())
                )
            }).map_err(|e| e.into());

            callback(result)
        })
    });
}

pub fn load_mime_table(mut env: JNIEnv) -> jobjectArray {
    match mime::load_extensions() {
        Ok(table) => {
            let java_table = env.new_object_array(table.len() as _, "Ljava/lang/String", JObject::null()).unwrap();
            for (i, (key, extensions)) in table.into_iter().enumerate() {
                let entry = env.new_object_array(extensions.len() as jsize + 1, "java/lang/String", JObject::null()).unwrap();

                let java_key = env.new_string(key).unwrap();
                env.set_object_array_element(&entry, 0, java_key).unwrap();
                for (j, ext) in extensions.into_iter().enumerate() {
                    let java_value = env.new_string(ext).unwrap();
                    env.set_object_array_element(&entry, (j + 1) as jsize, java_value).unwrap();
                }

                env.set_object_array_element(&java_table, i as jsize, entry).unwrap();
            }
            java_table.into_raw()
        }
        Err(err) => {
            env.throw_new("java/lang/RuntimeException", err.message()).unwrap();
            std::ptr::null_mut() as jobject
        }
    }
}

pub fn load_folders(mut env: JNIEnv) -> jobjectArray {
    match folders::load_folders() {
        Ok(folders) => {
            let java_folders = env.new_object_array(folders.len() as _, "java/lang/String", JObject::null()).unwrap();
            for (i, folder) in folders.into_iter().enumerate() {
                let java_folder = env.new_string(folder).unwrap();
                env.set_object_array_element(&java_folders, i as jsize, java_folder).unwrap();
            }
            java_folders.into_raw()
        }
        Err(err) => {
            env.throw_new("java/lang/RuntimeException", err.message()).unwrap();
            std::ptr::null_mut() as jobject
        }
    }
}