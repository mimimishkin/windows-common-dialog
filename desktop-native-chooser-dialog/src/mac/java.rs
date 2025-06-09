use jni::objects::{JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::{jboolean, jint, jlong, jsize};
use jni::JNIEnv;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSWindow};
use objc2_foundation::{NSArray, NSString, NSURL};
use std::sync::Arc;
use crate::mac::dialog::choose;
use crate::mac::params::MacChooserDialogParams;
use crate::mac::utils::{element_to_uttype, new_url, with_main_thread, ToString};

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_mimimishkin_common_chooser_dialog_NativeHelper_showMacOSChooserDialog0<'a>(
    mut env: JNIEnv<'a>,
    _class: JClass<'a>,
    title: JString<'a>,
    filter: JObjectArray<'a>,
    use_glob_patterns: jboolean,
    is_saver: jboolean,
    multiple: jboolean,
    initial_directory: JString<'a>,
    suggested_name: JString<'a>,
    owner: jlong,
    callback: JObject<'a>
) { unsafe {
    let title: Option<String> = (!title.is_null()).then(|| env.get_string_unchecked(&title).unwrap().into());
    let title: Option<Retained<NSString>> = title.map(|s| NSString::from_str(&s));
    let filter_size = env.get_array_length(&filter).unwrap() as usize;
    let filter = (0..filter_size).filter_map(|i| {
        let element = JString::from(env.get_object_array_element(&filter, i as _).unwrap());
        let element: String = env.get_string_unchecked(&element).unwrap().into();
        element_to_uttype(&element)
    }).collect::<Vec<_>>();
    let filter = NSArray::from_retained_slice(&filter);
    let glob_patterns = if use_glob_patterns != 0 {
        todo!()
    } else {
        vec![]
    };
    let is_saver = is_saver != 0;
    let multiple = multiple != 0;
    let initial_directory: Option<String> = (!initial_directory.is_null()).then(|| env.get_string_unchecked(&initial_directory).unwrap().into());
    let initial_directory: Option<Retained<NSURL>> = initial_directory.and_then(|s| new_url(&s));
    let suggested_name: Option<String> = (!suggested_name.is_null()).then(|| env.get_string_unchecked(&suggested_name).unwrap().into());
    let suggested_name: Option<Retained<NSString>> = suggested_name.map(|s| NSString::from_str(&s));
    let owner: Option<Retained<NSWindow>> = Retained::retain(owner as _);

    let params = MacChooserDialogParams {
        title,
        filter,
        glob_patterns,
        is_saver,
        multiple,
        initial_directory,
        suggested_name,
        owner,
    };
    
    let params = Arc::new(params);

    let jvm = env.get_java_vm().unwrap();
    let callback = env.new_global_ref(callback).unwrap();
    
    choose(params, move |result| {
        let mut env = jvm.attach_current_thread().unwrap();

        match result {
            Ok((selection, last_dir)) => {
                let j_last_dir = last_dir.and_then(|url| env.new_string(url.to_string()).ok()).map_or(JObject::null(), |dir| JObject::from(dir));
                let j_selection = env.new_object_array(selection.len() as jsize, "java/lang/String", JObject::null()).unwrap();
                for (i, url) in selection.iter().enumerate() {
                    let j_url = env.new_string(url.to_string()).unwrap();
                    env.set_object_array_element(&j_selection, i as jsize, &j_url).unwrap();
                }

                env.call_method(
                    callback,
                    "onSuccess",
                    "(Ljava/lang/String;[Ljava/lang/String;)V",
                    &[
                        JValue::Object(&j_last_dir),
                        JValue::Object(&j_selection)
                    ]
                ).unwrap();
            }
            Err(err) => {
                let message = env.new_string(err.0).unwrap();
                env.call_method(
                    callback,
                    "onError",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(&message)]
                ).unwrap();
            }
        }
    });
} }

fn find_window_with_nested(
    windows: Retained<NSArray<NSWindow>>,
    title: Option<&NSString>,
    pos: &Option<(f64, f64)>,
    size: &Option<(f64, f64)>,
    max_depth: i32,
    cur_depth: i32,
) -> Option<(Retained<NSWindow>, f64)> {
    let mut best = None;
    let mut best_error = f64::MAX;

    for window in windows {
        let title_match = title.map(|title| unsafe { window.title().isEqualToString(title) }).unwrap_or_default();

        if let Some((px, py)) = pos {
            let bounds = window.frame();
            let cx = bounds.origin.x;
            let cy = bounds.origin.y;
            let mut error = (cx - px).abs() + (cy - py).abs();

            if let Some((sw, sh)) = size {
                let cw = bounds.size.width;
                let ch = bounds.size.height;
                error += (cw - sw).abs() + (ch - sh).abs();
            }

            if error < best_error {
                best = Some(window.clone());
                best_error = error;

                if error < 0.1 {
                    return Some((window, error));
                }
            }
        } else if title_match {
            // if the position is not specified, we can just take the first match
            return Some((window, 0.0));
        }

        if cur_depth < max_depth {
            if let Some(children) = unsafe { window.childWindows() } {
                if let Some(res) = find_window_with_nested(children, title, pos, size, max_depth, cur_depth + 1) {
                    if res.1 < 0.1 {
                        return Some(res);
                    } else if res.1 < best_error {
                        best = Some(res.0);
                        best_error = res.1;
                    }
                }
            }
        }
    }

    best.map(|b| (b, best_error))
}

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
    let pos = (match_position != 0).then_some((x as f64, y as f64));
    let size = (match_size != 0).then_some((width as f64, height as f64));

    let window = with_main_thread(|mtm| {
        let title: Option<Retained<NSString>> = title.map(|title| NSString::from_str(&title));
        let windows = NSApplication::sharedApplication(mtm).windows();
        find_window_with_nested(windows, title.as_deref(), &pos, &size, max_depth, 0)
            .take_if(|(_, error)| title.is_some() || *error < 0.1)
            .map(|(window, _)| window.as_ref() as *const NSWindow as isize)
    }).flatten().unwrap_or_default();

    window as jlong
} }