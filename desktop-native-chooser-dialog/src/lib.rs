//! This module provides a cross-platform file chooser dialog interface.
//! It shouldn't be used directly, it's only for outer kotlin project.

extern crate core;

#[cfg(target_os = "macos")]
pub mod mac;

mod dialog;
pub use dialog::*;

#[cfg(feature = "java")]
mod java;
#[cfg(feature = "java")]
pub use java::*;
