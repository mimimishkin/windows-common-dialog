use crate::mac::error::{MacDialogError, MacRes};
use objc2::ClassType;
use objc2::__framework_prelude::Retained;
use objc2_app_kit::{NSModalResponse, NSModalResponseCancel, NSModalResponseOK, NSOpenPanel, NSSavePanel, NSWindow};
use objc2_foundation::NSURL;
use std::sync::Arc;
use crate::mac::glob::GlobFilterDelegate;
use crate::mac::params::MacChooserDialogParams;
use crate::mac::utils::with_main_thread;

pub fn choose(params: Arc<MacChooserDialogParams>, callback: impl FnOnce(MacRes<(Vec<Retained<NSURL>>, Option<Retained<NSURL>>)>)) { unsafe {
    with_main_thread(|mtm| {
        fn begin_panel(
            panel: &NSSavePanel, 
            owner: Option<&NSWindow>, 
            callback: impl Fn(NSModalResponse) + Clone + 'static
        ) {
            unsafe {
                let block = block2::StackBlock::new(callback);
                if let Some(owner) = owner {
                    panel.beginSheetModalForWindow_completionHandler(owner, &block);
                } else {
                    panel.beginWithCompletionHandler(&block);
                }
            }
        }
        
        if params.is_saver {
            let panel = NSSavePanel::savePanel(mtm);

            panel.setTitle(params.title.as_deref());
            panel.setAllowedContentTypes(&params.filter);
            panel.setDirectoryURL(params.initial_directory.as_deref());
            if let Some(ref suggested_name) = params.suggested_name {
                panel.setNameFieldStringValue(suggested_name);
            }

            let callback = |r: NSModalResponse| match r {
                1 /* NSModalResponseOK */ | 0 /* NSModalResponseCancel */ => {
                    let selection = panel.URL().into_iter().collect::<Vec<_>>();
                    let last_folder = panel.directoryURL();

                    callback(Ok((selection, last_folder)));
                }

                code => {
                    let message = format!("NSSavePanel completed with code: {code}");
                    callback(Err(MacDialogError(message)));
                }
            };

            begin_panel(panel.as_ref(), params.owner.as_deref(), callback);
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

            let callback = |r: NSModalResponse| match r {
                1 /* NSModalResponseOK */ | 0 /* NSModalResponseCancel */ => {
                    let selection = panel.URLs().into_iter().collect::<Vec<_>>();
                    let last_folder = panel.directoryURL();

                    callback(Ok((selection, last_folder)));
                }

                code => {
                    let message = format!("NSOpenPanel completed with code: {code}");
                    callback(Err(MacDialogError(message)));
                }
            };

            begin_panel(panel.as_ref(), params.owner.as_deref(), callback);
        };
    });
} }
