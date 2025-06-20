#[cfg(target_os = "windows")]
pub mod win;
#[cfg(all(feature = "c_binding", target_os = "windows"))]
pub use win::c_binding::*;
#[cfg(all(feature = "java", target_os = "windows"))]
pub use win::java::*;

#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(all(feature = "c_binding", target_os = "macos"))]
pub use mac::c_binding::*;
#[cfg(all(feature = "java", target_os = "macos"))]
pub use mac::java::*;

#[cfg(feature = "java")]
mod awt_window;

#[cfg(feature = "java")]
pub use awt_window::*;
