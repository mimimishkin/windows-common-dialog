//! Manual test: well-known folders and MIME lookups (no dialog involved).
//!
//! Run with `cargo run --example known_folders_and_mime`.

use windows_common_dialog::{folders, mime};

fn main() {
    println!("-- well-known folders --");
    match folders::load_folders() {
        Ok(paths) => {
            for path in paths {
                println!("  {path}");
            }
        }
        Err(err) => eprintln!("  failed: {err}"),
    }

    println!("-- extension -> type --");
    for extension in [".txt", ".md", ".rs", ".png", ".nonexistent"] {
        println!("  {extension} -> {:?}", mime::type_for_extension(extension));
    }

    println!("-- type -> extensions --");
    match mime::extensions_for_type("text", "plain") {
        Ok(extensions) => println!("  text/plain -> {extensions:?}"),
        Err(err) => eprintln!("  failed: {err}"),
    }
    match mime::extensions_for_type("image", "*") {
        Ok(extensions) => println!("  image/* -> {} extension(s)", extensions.len()),
        Err(err) => eprintln!("  failed: {err}"),
    }
}
