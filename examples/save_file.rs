//! Manual test: save a file.
//!
//! Run with `cargo run --example save_file`, pick a folder and a name, then confirm
//! the overwrite prompt. The file is NOT created — only the chosen path is printed.

use windows_common_dialog::dialog::choose;
use windows_common_dialog::file_filter::WinFileFilter;
use windows_common_dialog::params::{ChoosingMode, WinChooserDialogParams};
use windows_common_dialog::utils::{self, ShellItemEx, WinRes};

fn main() {
    if let Err(err) = utils::with_com(run) {
        eprintln!("failed: {err}");
    }
}

fn run() -> WinRes<()> {
    let params = WinChooserDialogParams {
        title: Some("Save the document as".into()),
        mode: ChoosingMode::Saving,
        suggested_name: Some("untitled.txt".into()),
        filters: vec![WinFileFilter::new("Text documents", "*.txt")],
        ..Default::default()
    };

    let (items, _) = choose(&params)?;
    match items.first() {
        Some(item) => println!("would save to: {}", item.absolute_parsing_name()),
        None => println!("cancelled"),
    }
    Ok(())
}
