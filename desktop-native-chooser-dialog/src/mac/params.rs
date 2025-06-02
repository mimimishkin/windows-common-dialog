use objc2::rc::Retained;
use objc2_app_kit::NSWindow;
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_uniform_type_identifiers::UTType;

pub struct MacChooserDialogParams {
    pub title: Option<Retained<NSString>>,
    pub filter: Retained<NSArray<UTType>>,
    pub glob_patterns: Vec<String>,
    pub is_saver: bool,
    pub multiple: bool,
    pub initial_directory: Option<Retained<NSURL>>,
    pub suggested_name: Option<Retained<NSString>>,
    pub owner: Option<Retained<NSWindow>>,
}