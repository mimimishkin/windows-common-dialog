//! Manual test: open one file.
//!
//! Run with `cargo run --example open_file`, then pick a file or press Cancel.

use windows_common_dialog::dialog::choose;
use windows_common_dialog::file_filter::WinFileFilter;
use windows_common_dialog::params::{ChoosingMode, WinChooserDialogParams};
use windows_common_dialog::utils::{self, ShellItemEx, WinRes};

fn main() {
    // The picked items belong to the STA that ran the dialog, so every later COM
    // call has to happen inside that same apartment.
    if let Err(err) = utils::with_com(run) {
        eprintln!("failed: {err}");
    }
}

fn run() -> WinRes<()> {
    let params = WinChooserDialogParams {
        title: Some("Pick a single file".into()),
        mode: ChoosingMode::FilesOnly,
        multiple: false,
        filters: vec![
            WinFileFilter::new("Text documents", "*.txt;*.md"),
            WinFileFilter::new("Rust sources", "*.rs"),
        ],
        ..Default::default()
    };

    let (items, folder) = choose(&params)?;
    if items.is_empty() {
        println!("cancelled");
    } else {
        for item in &items {
            println!("name:      {}", item.relative_name());
            println!("path:      {}", item.absolute_parsing_name());
        }
    }
    if let Some(folder) = folder {
        println!("was in:    {}", folder.absolute_parsing_name());
    }
    Ok(())
}
