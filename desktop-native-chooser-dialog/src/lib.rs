//! This module provides a cross-platform file chooser dialog interface.
//! It shouldn't be used directly, it's only for outer kotlin project.

#[cfg(target_os = "windows")]
pub mod win;
#[cfg(all(feature = "java", target_os = "windows"))]
pub use win::java::*;

#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(all(feature = "java", target_os = "macos"))]
pub use mac::java::*;

#[cfg(feature = "java")]
mod awt_window;
#[cfg(feature = "java")]
pub use awt_window::*;
