//! Windows common file dialogs with a few ergonomic additions:
//!
//! * [`dialog::choose`] wraps the native `IFileOpenDialog` / `IFileSaveDialog` and, among
//!   other things, allows picking files and folders *at the same time*, which the native
//!   dialog cannot do on its own.
//! * [`mime`] maps file extensions to/from MIME content types through the registry.
//! * [`folders`] resolves well-known folder paths; [`file_filter`] describes the file
//!   type filters of a dialog; [`utils`] provides low-level shell-item helpers.

pub mod dialog;
pub mod file_filter;
pub mod folders;
pub mod mime;
pub mod params;
pub mod strings;
pub mod utils;

mod fnf_events;
