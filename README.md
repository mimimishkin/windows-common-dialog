# windows-common-dialog

Rust wrapper around the Windows common file dialogs (`IFileOpenDialog` / `IFileSaveDialog`)
with a friendlier API and a few things the native dialogs cannot do on their own.

Written in Rust, for Windows, using the `windows` crate.

## Why

The stock Windows dialogs work, but using them from Rust is awkward: COM boilerplate,
raw `HSTRING` and `PCWSTR` juggling, and shell items that are painful to turn into
strings. This crate keeps the native look and feel of the system dialogs and removes
the surrounding friction.

## Install

```toml
[dependencies]
windows-common-dialog = "0.1.0"
```

## Usage

Pick one file:

```rust
use windows_common_dialog::dialog::choose;
use windows_common_dialog::file_filter::WinFileFilter;
use windows_common_dialog::params::{ChoosingMode, WinChooserDialogParams};
use windows_common_dialog::utils::{self, ShellItemEx};

fn main() {
    utils::with_com(|| {
        let params = WinChooserDialogParams {
            title: Some("Pick a file".into()),
            mode: ChoosingMode::FilesOnly,
            filters: vec![WinFileFilter::new("Text documents", "*.txt;*.md")],
            ..Default::default()
        };

        let (items, _) = choose(&params)?;
        for item in &items {
            println!("{}", item.absolute_parsing_name());
        }
        Ok(())
    })
    .unwrap();
}
```

## Modes

`ChoosingMode` selects what the dialog accepts:

| Mode                  | What you get                                  |
|-----------------------|-----------------------------------------------|
| `FilesOnly`           | One or more existing files                    |
| `DirectoriesOnly`     | One or more folders                           |
| `FilesAndDirectories` | Files and folders mixed in one selection      |
| `Saving`              | A destination path for a new or existing file |

`FilesAndDirectories` is the main extension over the native dialogs. It is implemented
by the crate rather than by Windows.

## Parameters

`WinChooserDialogParams` covers the usual needs, and every field is optional:

- `title` — dialog caption; the system default is used when omitted.
- `mode` — see the table above.
- `multiple` — allow selecting several items at once.
- `filters` — named wildcard groups shown in the type dropdown, e.g.
  `("Images", "*.png;*.jpg;*.bmp")`. Empty means "all files".
- `initial_directory` — the folder the dialog opens in.
- `suggested_name` — the pre-filled file name, useful for save dialogs.
- `id` — a client GUID used to remember the dialog's state between sessions.
- `owner` — an optional parent window handle.

## Results

`choose` returns the selected items and, when it is known, the folder the dialog was
showing. An empty selection means the user pressed Cancel — that is not an error.
Selected items are shell items with a few convenience methods:

```rust
item.relative_name();          // "report.pdf"
item.absolute_parsing_name();  // "C:\Users\me\report.pdf"
item.is_file()?;
item.is_directory()?;
item.size()?;
item.content_type()?;          // "application/pdf"
item.link_target()?;           // for .lnk shortcuts
item.iter_children()?;         // direct children of a folder
```

## Com

The dialogs require COM, and the items you get back belong to the apartment that ran
the dialog. Do all COM work in one place:

```rust
utils::with_com(|| {
    let (items, _) = choose(&params)?;
    // read items here
    Ok(())
})
```

Initializing COM once and keeping it for the whole run of a real application is fine
too, using `init_com` / `quit_com`.

## Extra helpers

Two small utilities that are useful alongside the dialogs:

- `folders::load_folders()` — paths of the well-known folders (Desktop, Documents,
  Downloads, Music, Pictures, Videos).
- `mime::type_for_extension(".txt")` and `mime::extensions_for_type("text", "plain")` —
  extension/MIME lookups based on what Windows itself has registered, so the answers
  match the machine the app runs on.

## Examples

The `examples/` directory contains runnable manual tests:

```bash
cargo run --example open_file
cargo run --example open_files_multiple
cargo run --example save_file
cargo run --example open_folder
cargo run --example open_file_or_folder
cargo run --example inspect_item
cargo run --example known_folders_and_mime
```

Each dialog example opens a real dialog, prints what you picked, and returns
`cancelled` when you press Cancel — handy for eyeballing behaviour before wiring it
into an application.
