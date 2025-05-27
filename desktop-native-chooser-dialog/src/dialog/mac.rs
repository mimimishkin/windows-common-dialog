use crate::dialog::ChooserRes;
use crate::mac::with_main_thread;
use crate::{ChooserDialogOwner, ChooserDialogParams, ChooserDialogResult, ChoosingMode};
use dispatch2::run_on_main;
use objc2::ffi::nil;
use objc2::rc::{autoreleasepool, AutoreleasePool, Retained};
use objc2::runtime::{AnyObject, Bool, NSObject, NSObjectProtocol};
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSModalResponse, NSModalResponseCancel, NSModalResponseOK, NSOpenPanel, NSOpenSavePanelDelegate, NSSavePanel, NSView, NSWindow};
use objc2_core_foundation::{CFArray, CFDictionary, CFRetained, CFString, CGFloat};
use objc2_foundation::{NSArray, NSAutoreleasePool, NSMutableArray, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;
use std::cell::OnceCell;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use ChoosingMode::*;
// TODO: compatibility: now, only macOS 10.7+ is supported

fn match_glob(pat: &[char], text: &[char]) -> bool {
    let (mut pi, mut ti) = (0, 0);
    let (mut star_idx, mut match_idx) = (None, 0);

    while ti < text.len() {
        if pi < pat.len() && (pat[pi] == '?' || pat[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pat.len() && pat[pi] == '*' {
            star_idx = Some(pi);
            match_idx = ti;
            pi += 1;
        } else if let Some(si) = star_idx {
            pi = si + 1;
            match_idx += 1;
            ti = match_idx;
        } else {
            return false;
        }
    }

    while pi < pat.len() && pat[pi] == '*' {
        pi += 1;
    }

    pi == pat.len()
}

fn try_map_ext<T>(pattern: &str, process: impl Fn(&str) -> Option<T>) -> Option<T> {
    let full_i = pattern.rfind("*.");
    let mut res = full_i.and_then(|i| process(&pattern[i + 2..]));

    if res.is_none() {
        let i = pattern.rfind('.').filter(|&i| full_i.is_none() || i != full_i.unwrap() + 1);
        res = i.and_then(|i| process(&pattern[i + 1..]));
    }

    res
}

#[derive(Debug)]
struct FilterDelegateIvars {
    patterns: Vec<[char]>
}

impl FilterDelegateIvars {
    fn new(elements: &str) -> Self {
        Self {
            patterns: elements
                .split('\0')
                .filter_map(|s| (!s.contains('/')).then(|| s.chars().collect()))
                .collect()
        }
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = FilterDelegateIvars]
    struct FilterDelegate;

    unsafe impl NSObjectProtocol for FilterDelegate {}

    unsafe impl NSOpenSavePanelDelegate for FilterDelegate {
        #[unsafe(method(panel:shouldEnableURL:))]
        #[allow(non_snake_case)]
        unsafe fn panel_shouldEnableURL(&self, _sender: &AnyObject, url: &NSURL) -> bool {
            if let Some(name) = url.lastPathComponent() {
                let patterns = &self.ivars().patterns
                let name = autoreleasepool(|p| name.to_str(p).chars().collect())
                patterns.iter().any(|glob| match_glob(glob, &name))
            } else {
                true
            }
        }
    }
);

impl FilterDelegate {
    fn new(mtm: MainThreadMarker, elements: &str) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FilterDelegateIvars::new(elements));
        // SAFETY: The signature of `NSObject`'s `init` method is correct.
        unsafe { msg_send![super(this), init] }
    }
}

pub fn choose(params: Arc<ChooserDialogParams>, callback: impl FnOnce(ChooserRes<ChooserDialogResult>)) { unsafe {
    unsafe fn url_to_string(url: Retained<NSURL>) -> String {
        url.absoluteString().unwrap().to_string()
    }

    unsafe fn element_to_uttype(element: &str) -> Option<Retained<UTType>> {
        if element.contains('/') {
            let mime = NSString::from_str(&element);
            UTType::typeWithMIMEType(&mime)
        } else {
            try_map_ext(element, |ext| UTType::typeWithFilenameExtension(&NSString::from_str(ext)))
        }
    }

    unsafe fn str_to_url(s: &str) -> Option<&NSURL> {
        if s.contains("://") {
            // already a url
            NSURL::URLWithString(&NSString::from_str(s)).as_deref()
        } else {
            NSURL::from_directory_path(Path::new(s)).as_deref()
        }
    }

    with_main_thread(|mtm| {
        let owner = match params.owner {
            ChooserDialogOwner::NSWindow { ptr } => {
                Some(&*(ptr as *const NSWindow))
            }
            _ => None
        };

        let (panel, callback): (&NSSavePanel, fn(NSModalResponse)) = match params.mode {
            Saving => {
                let panel = NSSavePanel::savePanel(mtm);

                // todo id
                panel.setTitle(params.title.as_ref().map(|t| NSString::from_str(&t).deref()));
                params.filters.first().inspect(|filter| {
                    let types = filter.elements().filter_map(element_to_uttype);
                    let types = NSArray::from_slice(&types);
                    panel.setAllowedContentTypes(&types);
                });
                panel.setDirectoryURL(params.initial_directory.as_ref().and_then(str_to_url));
                if params.suggested_name.is_some() {
                    let suggested_name = NSString::from_str(params.suggested_name.as_ref().unwrap());
                    panel.setNameFieldStringValue(&suggested_name);
                }

                let callback = |r: NSModalResponse| {
                    match r {
                        NSModalResponseOK | NSModalResponseCancel => {
                            let result = panel.URL().and_then(url_to_string);
                            let result = result.into_iter().collect();
                            let last_folder = panel.directoryURL().and_then(url_to_string);

                            callback(Ok(ChooserDialogResult::new(result, last_folder)));
                        }

                        code => {
                            callback(Err(format!("NSSavePanel completed with code: {code}").into()));
                        }
                    }
                };

                (panel.as_ref(), callback)
            },

            _ => {
                let panel = NSOpenPanel::openPanel(mtm);

                // todo id
                panel.setTitle(params.title.as_ref().map(|t| NSString::from_str(&t).deref()));
                if let DirectoriesOnly = params.mode {
                    panel.setCanChooseDirectories(true);
                    panel.setCanChooseFiles(false);
                } else {
                    panel.setCanChooseDirectories(params.mode == FilesAndDirectories);
                    panel.setCanChooseFiles(true);

                    params.filters.first().inspect(|filter| {
                        let types = filter.elements().filter_map(element_to_uttype);
                        let types = NSArray::from_slice(&types);
                        panel.setAllowedContentTypes(&types);

                        let filter_delegate = FilterDelegate::new(mtm, filter.elements_inlined());
                        panel.setDelegate(Some(&filter_delegate));
                    });
                }
                panel.setAllowsMultipleSelection(params.multiple);
                panel.setDirectoryURL(params.initial_directory.as_ref().and_then(str_to_url));
                if params.suggested_name.is_some() {
                    let suggested_name = NSString::from_str(params.suggested_name.as_ref().unwrap());
                    panel.setNameFieldStringValue(&suggested_name);
                }

                let callback = |r: NSModalResponse| {
                    match r {
                        NSModalResponseOK | NSModalResponseCancel => {
                            let result = panel.URLs().iter().map(url_to_string).collect::<Vec<_>>();
                            let last_folder = panel.directoryURL().and_then(url_to_string);

                            callback(Ok(ChooserDialogResult::new(result, last_folder)));
                        }

                        code => {
                            callback(Err(format!("NSOpenPanel completed with code: {code}").into()));
                        }
                    }
                };

                (panel.as_super(), callback)
            }
        };

        if let Some(owner) = owner {
            panel.beginSheetModalForWindow_completionHandler(&owner, &callback);
        } else {
            panel.beginWithCompletionHandler(&callback);
        }
    });
} }
