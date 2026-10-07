//! Manual test: pick a folder.
//!
//! Run with `cargo run --example open_folder`, then select a folder or press Cancel.

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
        title: Some("Select a folder".into()),
        mode: ChoosingMode::DirectoriesOnly,
        ..Default::default()
    };

    let (items, folder) = choose(&params)?;
    if let Some(item) = items.first() {
        println!("folder: {}", item.absolute_parsing_name());
        if let Ok(children) = item.iter_children() {
            println!("contains {} item(s)", children.count());
        }
    } else {
        println!("cancelled");
    }
    if let Some(folder) = folder {
        println!("was in: {}", folder.absolute_parsing_name());
    }
    Ok(())
}
