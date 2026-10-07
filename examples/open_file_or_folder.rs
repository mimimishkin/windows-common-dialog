//! Manual test: the hybrid dialog that picks files *and* folders in one go.
//!
//! This is the feature the native dialog cannot do on its own.
//! Run with `cargo run --example open_file_or_folder`, then pick a mix of files and
//! folders with Ctrl-click and confirm.

use windows_common_dialog::dialog::choose;
use windows_common_dialog::params::{ChoosingMode, WinChooserDialogParams};
use windows_common_dialog::utils::{self, ShellItemEx, WinRes};

fn main() {
    if let Err(err) = utils::with_com(run) {
        eprintln!("failed: {err}");
    }
}

fn run() -> WinRes<()> {
    let params = WinChooserDialogParams {
        mode: ChoosingMode::FilesAndDirectories,
        multiple: true,
        ..Default::default()
    };

    let (items, _) = choose(&params)?;
    println!("selected {} item(s)", items.len());
    for item in &items {
        let kind = match (item.is_file(), item.is_directory()) {
            (Ok(true), _) => "file",
            (_, Ok(true)) => "folder",
            _ => "?",
        };
        println!("  [{kind}] {}", item.absolute_parsing_name());
    }
    Ok(())
}
