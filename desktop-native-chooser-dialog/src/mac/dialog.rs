use crate::mac::error::MacRes;
use crate::{with_main_thread, GlobFilterDelegate, MacChooserDialogParams};
use objc2::ClassType;
use objc2::__framework_prelude::Retained;
use objc2_app_kit::{NSModalResponse, NSModalResponseCancel, NSModalResponseOK, NSOpenPanel, NSSavePanel};
use objc2_foundation::NSURL;
use std::sync::Arc;

pub fn choose(params: Arc<MacChooserDialogParams>, callback: impl FnOnce(MacRes<(Vec<Retained<NSURL>>, Option<Retained<NSURL>>)>)) { unsafe {
    with_main_thread(|mtm| {
        let (panel, callback): (&NSSavePanel, fn(NSModalResponse)) = if params.is_saver {
            let panel = NSSavePanel::savePanel(mtm);

            panel.setTitle(params.title.as_deref());
            panel.setAllowedContentTypes(&params.filter);
            panel.setDirectoryURL(params.initial_directory.as_deref());
            if let Some(ref suggested_name) = params.suggested_name {
                panel.setNameFieldStringValue(suggested_name);
            }

            let callback = |r: NSModalResponse| {
                match r {
                    NSModalResponseOK | NSModalResponseCancel => {
                        let selection = panel.URL().into_iter().collect::<Vec<_>>();
                        let last_folder = panel.directoryURL();

                        callback(Ok((selection, last_folder)));
                    }

                    code => {
                        callback(Err(format!("NSSavePanel completed with code: {code}").into()));
                    }
                }
            };

            (panel.as_ref(), callback)
        } else {
            let panel = NSOpenPanel::openPanel(mtm);

            panel.setTitle(params.title.as_deref());
            panel.setAllowedContentTypes(&params.filter);
            if !params.glob_patterns.is_empty() {
                let filter_delegate = GlobFilterDelegate::new(mtm, &params.glob_patterns);
                panel.setDelegate(Some(&filter_delegate));
            }
            panel.setAllowsMultipleSelection(params.multiple);
            panel.setDirectoryURL(params.initial_directory.as_deref());
            if let Some(ref suggested_name) = params.suggested_name {
                panel.setNameFieldStringValue(suggested_name);
            }

            let callback = |r: NSModalResponse| {
                match r {
                    NSModalResponseOK | NSModalResponseCancel => {
                        let selection = panel.URLs().into_iter().collect::<Vec<_>>();
                        let last_folder = panel.directoryURL();

                        callback(Ok((selection, last_folder)));
                    }

                    code => {
                        callback(Err(format!("NSOpenPanel completed with code: {code}").into()));
                    }
                }
            };

            (panel.as_super(), callback)
        };

        if let Some(ref owner) = params.owner {
            panel.beginSheetModalForWindow_completionHandler(owner, &callback);
        } else {
            panel.beginWithCompletionHandler(&callback);
        }
    });
} }
