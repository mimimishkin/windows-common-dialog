use crate::win::dialog::choose;
use crate::win::file_filter::WinFileFilter;
use crate::win::folders::load_folders;
use crate::win::mime::load_extensions;
use crate::win::params::{ChoosingMode, WinChooserDialogParams};
use crate::win::utils::{with_com, ShellItemEx};
use jni::objects::{JClass, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong, jobject, jobjectArray, jsize};
use jni::JNIEnv;
use std::ops::Deref;
use windows::core::{GUID, HSTRING};
use windows::Win32::Foundation::{FALSE, HWND, LPARAM, RECT, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW};
use windows_core::BOOL;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_showWindowsChooserDialog0<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass<'a>,
    id_part1: jlong,
    id_part2: jlong,
    title: JString<'a>,
    filter_names: JObjectArray<'a>,
    filter_patterns: JObjectArray<'a>,
    mode: jint,
    multiple: jboolean,
    initial_directory: JString<'a>,
    suggested_name: JString<'a>,
    owner: jlong,
) -> jobjectArray { unsafe {
    let id = ((id_part1 as u128) << 64) | (id_part2 as u128);
    let id: Option<GUID> = (id != 0).then_some(id.into());
    let title: Option<String> = (!title.is_null()).then(|| env.get_string_unchecked(&title).unwrap().into());
    let title: Option<HSTRING> = title.map(HSTRING::from);
    let filters_size = env.get_array_length(&filter_names).unwrap() as usize;
    let filters = (0..filters_size).map(|i| {
        let name = JString::from(env.get_object_array_element(&filter_names, i as _).unwrap());
        let name: String = env.get_string_unchecked(&name).unwrap().into();
        let patterns = JString::from(env.get_object_array_element(&filter_patterns, i as _).unwrap());
        let patterns: String = env.get_string_unchecked(&patterns).unwrap().into();
        WinFileFilter {
            name: name.into(),
            patterns: patterns.into(),
        }
    }).collect::<Vec<_>>();
    let mode = match mode {
        0 => ChoosingMode::Saving,
        1 => ChoosingMode::FilesOnly,
        2 => ChoosingMode::DirectoriesOnly,
        3 => ChoosingMode::FilesAndDirectories,
        _ => unreachable!()
    };
    let multiple = multiple != 0;
    let initial_directory: Option<String> = (!initial_directory.is_null()).then(|| env.get_string_unchecked(&initial_directory).unwrap().into());
    let initial_directory: Option<HSTRING> = initial_directory.map(HSTRING::from);
    let suggested_name: Option<String> = (!suggested_name.is_null()).then(|| env.get_string_unchecked(&suggested_name).unwrap().into());
    let suggested_name: Option<HSTRING> = suggested_name.map(HSTRING::from);
    let owner = (owner != 0).then_some(HWND(owner as _));

    let params = WinChooserDialogParams {
        id,
        title,
        filters,
        mode,
        multiple,
        initial_directory,
        suggested_name,
        owner,
    };

    with_com(|| {
        match choose(&params) {
            Ok((selection, folder)) => {
                let j_result = env.new_object_array((selection.len() + 1) as jsize, "java/lang/String", JObject::null()).unwrap();
                for (i, item) in selection.into_iter().enumerate() {
                    let j_item = env.new_string(item.absolute_parsing_name().to_string()).unwrap();
                    env.set_object_array_element(&j_result, (i + 1) as jsize, j_item).unwrap();
                }

                if let Some(folder) = folder {
                    let j_folder = env.new_string(folder.absolute_parsing_name().to_string()).unwrap();
                    env.set_object_array_element(&j_result, 0, j_folder).unwrap();
                }

                j_result.into_raw()
            }
            Err(err) => {
                env.throw_new("java/lang/Exception", err.message()).unwrap();
                std::ptr::null_mut() as jobject
            }
        }
    })
} }

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_loadMimeTable0<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> jobjectArray {
    match load_extensions() {
        Ok(table) => {
            let j_table = env.new_object_array(table.len() as _, "[Ljava/lang/String;", JObject::null()).unwrap();
            for (i, (key, extensions)) in table.into_iter().enumerate() {
                let entry = env.new_object_array(extensions.len() as jsize + 1, "java/lang/String", JObject::null()).unwrap();

                let j_key = env.new_string(key).unwrap();
                env.set_object_array_element(&entry, 0, j_key).unwrap();
                for (j, ext) in extensions.into_iter().enumerate() {
                    let j_value = env.new_string(ext).unwrap();
                    env.set_object_array_element(&entry, (j + 1) as jsize, j_value).unwrap();
                }

                env.set_object_array_element(&j_table, i as jsize, entry).unwrap();
            }
            j_table.into_raw()
        }
        Err(err) => {
            env.throw_new("java/lang/Exception", err.message()).unwrap();
            std::ptr::null_mut() as jobject
        }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_loadDirectories0<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> jobjectArray {
    match load_folders() {
        Ok(folders) => {
            let java_folders = env.new_object_array(folders.len() as _, "java/lang/String", JObject::null()).unwrap();
            for (i, folder) in folders.into_iter().enumerate() {
                let java_folder = env.new_string(folder).unwrap();
                env.set_object_array_element(&java_folders, i as jsize, java_folder).unwrap();
            }
            java_folders.into_raw()
        }
        Err(err) => {
            env.throw_new("java/lang/Exception", err.message()).unwrap();
            std::ptr::null_mut() as jobject
        }
    }
}

#[derive(Debug)]
struct FindWindowInfo<'a> {
    hwnd: HWND,
    title: Option<&'a HSTRING>,
    buffer: &'a mut [u16],
    left_top: Option<(i32, i32)>,
    right_bottom: Option<(i32, i32)>,
    error: u32,
    max_depth: i32,
    cur_depth: i32,
}

extern "system" fn find_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL { unsafe {
    let info = &mut *(lparam.0 as *mut FindWindowInfo);

    // check title or ignore if not specified
    let title_match = info.title.map(|title| {
        let title_len = title.len();
        let len = GetWindowTextLengthW(hwnd) as usize;
        if len != title_len {
            false
        } else if title_len != 0 && GetWindowTextW(hwnd, info.buffer) == 0 {
            false
        } else {
            title.deref() == &info.buffer[..len]
        }
    }).unwrap_or_default();

    if let Some((left, top)) = info.left_top {
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let mut new_error = rect.left.abs_diff(left) + rect.top.abs_diff(top);

        if let Some((right, bottom)) = info.right_bottom {
            new_error += rect.right.abs_diff(right) + rect.bottom.abs_diff(bottom);
        }

        if new_error < info.error {
            info.hwnd = hwnd;
            info.error = new_error;

            if new_error == 0 {
                return FALSE;
            }
        }
    } else if title_match {
        // if the position is not specified, we can just take the first match
        info.hwnd = hwnd;
        info.error = 0;
        return FALSE;
    }

    if info.cur_depth < info.max_depth {
        info.cur_depth += 1;
        let _ = EnumChildWindows(Some(hwnd), Some(find_window_proc), LPARAM(info as *mut _ as _));
        info.cur_depth -= 1;
        if info.error == 0 {
            return FALSE;
        }
    }

    TRUE
} }

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_findWindowNativeHandle0<'a>(
    env: JNIEnv<'a>,
    _: JClass<'a>,
    title: JString<'a>,
    x: jint,
    y: jint,
    match_position: jboolean,
    width: jint,
    height: jint,
    match_size: jboolean,
    max_depth: jint
) -> jlong { unsafe {
    let title = env.get_string_unchecked(&title).ok();
    let title: Option<String> = title.map(|title| title.into());
    let title: Option<HSTRING> = title.map(|title| HSTRING::from(title));
    let left_top = (match_position != 0).then_some((x as i32, y as i32));
    let right_bottom = (match_size != 0).then(|| {
        let (x, y) = left_top.unwrap();
        (x + width as i32, y + height as i32)
    });

    let hwnd = with_com(|| {
        let mut info = FindWindowInfo {
            hwnd: HWND::default(),
            title: title.as_ref(),
            buffer: &mut vec![0u16; title.as_ref().map_or(0, |t| t.len() + 1)],
            left_top,
            right_bottom,
            error: u32::MAX,
            max_depth,
            cur_depth: 0,
        };

        let _ = EnumWindows(Some(find_window_proc), LPARAM(&mut info as *mut _ as _));

        // require strict match when we don't know the title
        if title.is_none() && info.error != 0 {
            return HWND::default();
        }

        info.hwnd
    });

    hwnd.0 as jlong
} }