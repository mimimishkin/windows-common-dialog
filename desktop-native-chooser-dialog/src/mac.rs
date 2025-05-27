use dispatch2::run_on_main;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

pub(crate) fn with_main_thread<R>(block: impl FnOnce(MainThreadMarker) -> R) -> Option<R> {
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