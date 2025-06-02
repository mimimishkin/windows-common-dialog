pub mod glob;
pub mod params;
pub mod utils;
pub mod dialog;
pub mod error;

#[cfg(feature = "java")]
pub mod java;

// TODO: compatibility: now, only macOS 10.7+ is supported