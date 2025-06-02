use dispatch2::run_on_main;
use objc2::MainThreadMarker;
use objc2::__framework_prelude::Retained;
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;
use std::path::Path;

pub fn with_main_thread<R>(block: impl FnOnce(MainThreadMarker) -> R) -> Option<R> {
    if let Some(mtm) = MainThreadMarker::new() {
        Some(block(mtm))
    } else {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let app = NSApplication::sharedApplication(mtm);
        if unsafe { app.isRunning() } {
            Some(run_on_main(block))
        } else {
            None
        }
    }
}

pub fn try_map_ext<T>(pattern: &str, process: impl Fn(&str) -> Option<T>) -> Option<T> {
    let full_i = pattern.rfind("*.");
    let mut res = full_i.and_then(|i| process(&pattern[i + 2..]));

    if res.is_none() {
        let i = pattern.rfind('.').filter(|&i| full_i.is_none() || i != full_i.unwrap() + 1);
        res = i.and_then(|i| process(&pattern[i + 1..]));
    }

    res
}

pub trait ToString {
    fn to_string(&self) -> String;
}

impl ToString for NSURL {
    fn to_string(&self) -> String {
        unsafe { self.absoluteString().unwrap().to_string() }
    }
}

pub fn element_to_uttype(element: &str) -> Option<Retained<UTType>> {
    if element.contains('/') {
        let mime = NSString::from_str(&element);
        unsafe { UTType::typeWithMIMEType(&mime) }
    } else {
        unsafe { try_map_ext(element, |ext| UTType::typeWithFilenameExtension(&NSString::from_str(ext))) }
    }
}

pub fn new_url(s: &str) -> Option<Retained<NSURL>> {
    if s.contains("://") {
        // already a url
        unsafe { NSURL::URLWithString(&NSString::from_str(s)) }
    } else {
        NSURL::from_directory_path(Path::new(s))
    }
}