//! Manual test: open several files at once (multi-select).
//!
//! Run with `cargo run --example open_files_multiple`, then Ctrl-click / Shift-click
//! several files.

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
        title: Some("Pick several files".into()),
        mode: ChoosingMode::FilesOnly,
        multiple: true,
        filters: vec![WinFileFilter::new("Any image", "*.png;*.jpg;*.jpeg;*.bmp")],
        ..Default::default()
    };

    let (items, _) = choose(&params)?;
    println!("selected {} item(s)", items.len());
    for item in &items {
        println!("  {}", item.absolute_parsing_name());
    }
    Ok(())
}
