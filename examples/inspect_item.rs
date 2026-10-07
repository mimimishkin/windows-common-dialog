//! Manual test: inspect a picked item in depth (size, MIME type, shortcut target).
//!
//! Run with `cargo run --example inspect_item`, then pick anything — a file, a folder,
//! or a `.lnk` shortcut to check the link target.

use windows::Win32::UI::Shell::IShellItem;
use windows::core::HSTRING;

use windows_common_dialog::dialog::choose;
use windows_common_dialog::params::{ChoosingMode, WinChooserDialogParams};
use windows_common_dialog::utils::{self, ShellItemEx, WinRes};

fn main() {
    if let Err(err) = utils::with_com(run) {
        eprintln!("failed: {err}");
    }
}

fn describe(item: &IShellItem) -> WinRes<()> {
    println!("name:      {}", item.relative_name());
    println!("path:      {}", item.absolute_parsing_name());
    println!("is file:   {:?}", item.is_file());
    println!("is folder: {:?}", item.is_directory());
    println!("size:      {:?}", item.size());
    println!("mime:      {:?}", item.content_type());

    match item.link_target() {
        Ok(target) => println!("link to:   {}", target.absolute_parsing_name()),
        Err(_) => println!("link to:   (not a link)"),
    }
    Ok(())
}

fn run() -> WinRes<()> {
    let params = WinChooserDialogParams {
        title: Some("Pick an item to inspect".into()),
        mode: ChoosingMode::FilesAndDirectories,
        ..Default::default()
    };

    let (items, _) = choose(&params)?;
    let Some(item) = items.first() else {
        println!("cancelled");
        return Ok(());
    };

    describe(&item)?;

    // Children of the picked item, resolved through the shell namespace.
    if let Ok(children) = item.iter_children() {
        for child in children.take(10) {
            println!("  child: {}", child.relative_name());
        }
    }

    // Navigation helper: resolves a relative path from the picked item.
    match utils::create_item_in(&item, &HSTRING::from("..")) {
        Ok(parent) => println!("parent:    {}", parent.absolute_parsing_name()),
        Err(err) => println!("parent:    lookup failed ({err})"),
    }
    Ok(())
}
