use objc2::__framework_prelude::Retained;
use objc2::rc::autoreleasepool;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::NSOpenSavePanelDelegate;
use objc2_foundation::NSURL;

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

#[derive(Debug)]
struct GlobFilterDelegateIvars {
    patterns: Vec<Vec<char>>
}

impl GlobFilterDelegateIvars {
    fn new(elements: &[String]) -> Self {
        Self {
            patterns: elements
                .into_iter()
                .filter_map(|s| (!s.contains('/')).then(|| s.chars().collect()))
                .collect()
        }
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = GlobFilterDelegateIvars]
    pub struct GlobFilterDelegate;

    unsafe impl NSObjectProtocol for GlobFilterDelegate {}

    unsafe impl NSOpenSavePanelDelegate for GlobFilterDelegate {
        #[unsafe(method(panel:shouldEnableURL:))]
        #[allow(non_snake_case)]
        unsafe fn panel_shouldEnableURL(&self, _sender: &AnyObject, url: &NSURL) -> bool { unsafe {
            if let Some(name) = url.lastPathComponent() {
                let patterns = &self.ivars().patterns;
                let name: Vec<char> = autoreleasepool(|p| name.to_str(p).chars().collect());
                patterns.iter().any(|glob| match_glob(glob, &name))
            } else {
                true
            }
        } }
    }
);

impl GlobFilterDelegate {
    pub fn new(mtm: MainThreadMarker, elements: &[String]) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(GlobFilterDelegateIvars::new(elements));
        unsafe { msg_send![super(this), init] }
    }
}