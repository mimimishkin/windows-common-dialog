use crate::*;
use jni::objects::{JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{jboolean, jint, jlong, jobjectArray, jsize};
use jni::JNIEnv;
use std::sync::Arc;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_isChooserAvailable<'a>(
    _env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> jboolean {
    1 // true
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_showChooserDialog<'a>(
    mut env: JNIEnv<'a>,
    obj: JObject<'a>,
    id_part1: jlong,
    id_part2: jlong,
    title: JString<'a>,
    filters: JObjectArray<'a>,
    mode: jint,
    multiple: jboolean,
    initial_directory: JString<'a>,
    suggested_name: JString<'a>,
    owner: jlong,
) { unsafe {
    let id = ((id_part1 as u128) << 64) | (id_part2 as u128);
    let id = (id != 0).then_some(id);
    let title: Option<String> = (!title.is_null()).then(|| env.get_string_unchecked(&title).unwrap().into());
    let filters_size = env.get_array_length(&filters).unwrap() as usize;
    let filters = (0..filters_size).map(|i| {
        let j_string = JString::from(env.get_object_array_element(&filters, i as _).unwrap());
        let str: String = env.get_string_unchecked(&j_string).unwrap().into();
        FileFilter(str)
    }).collect::<Vec<_>>();
    let mode = match mode {
        0 => ChoosingMode::Saving,
        1 => ChoosingMode::FilesOnly,
        2 => ChoosingMode::DirectoriesOnly,
        3 => ChoosingMode::FilesAndDirectories,
        _ => unreachable!()
    };
    let initial_directory: Option<String> = (!initial_directory.is_null()).then(|| env.get_string_unchecked(&initial_directory).unwrap().into());
    let suggested_name: Option<String> = (!suggested_name.is_null()).then(|| env.get_string_unchecked(&suggested_name).unwrap().into());
    let owner = {
        #[cfg(target_os = "windows")]
        { ChooserDialogOwner::HWND { ptr: owner as _ } }
        #[cfg(target_os = "macos")]
        { ChooserDialogOwner::NSWindow { ptr: owner as _ } }
    };

    let params = ChooserDialogParams {
        id,
        title: title.as_deref(),
        filters,
        mode,
        multiple: multiple != 0,
        initial_directory: initial_directory.as_deref(),
        suggested_name: suggested_name.as_deref(),
        owner,
    };

    let jvm = env.get_java_vm().unwrap();
    let obj = env.new_global_ref(obj).unwrap();

    choose(Arc::new(params), move |result| {
        // todo: this should be done at thread start (to support cancellation)
        let mut env = jvm.attach_current_thread().unwrap();

        match result {
            Ok(r) => {
                let selected = env.new_object_array(r.selected.len() as jsize, "java/lang/String", JObject::null()).unwrap();
                for (i, s) in r.selected.into_iter().enumerate() {
                    let s = env.new_string(s).unwrap();
                    env.set_object_array_element(&selected, i as jsize, s).unwrap();
                }

                let last_folder = r.last_folder.map_or(
                    JObject::null(),
                    |folder| env.new_string(folder).unwrap().into()
                );

                env.call_method(
                    &obj,
                    "onSuccess",
                    "(Ljava/lang/String;java/lang/String;)V",
                    &[JValue::Object(&selected), JValue::Object(&last_folder)]
                ).unwrap();
            }
            Err(e) => {
                let message = env.new_string(e.0).unwrap();
                env.call_method(
                    &obj,
                    "onError",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&message)]
                ).unwrap();
            }
        };
    });
} }

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_loadMimeTable<'a>(
    env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> jobjectArray {
    load_mime_table(env)
}

#[cfg(target_os = "windows")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_loadFolders<'a>(
    env: JNIEnv<'a>,
    _class: JClass<'a>,
) -> jobjectArray {
    load_folders(env)
}