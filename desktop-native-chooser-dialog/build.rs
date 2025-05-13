use cbindgen::{Config, Language};

fn main() {
    let config = Config {
        language: Language::C,
        ..Config::default()
    };
    cbindgen::generate_with_config("", config)
        .expect("Cannot create bindings")
        .write_to_file("desktop_native_chooser_dialog.h");
}
