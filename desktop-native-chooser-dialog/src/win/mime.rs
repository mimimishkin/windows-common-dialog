use crate::win::utils::WinRes;
use std::collections::HashMap;
use windows_registry::CLASSES_ROOT;

pub fn load_extensions() -> WinRes<HashMap<String, Vec<String>>> {
    let mut table: HashMap<String, Vec<String>> = HashMap::new();

    for ext in CLASSES_ROOT.keys()? {
        if !ext.starts_with('.') {
            continue
        }

        if let Ok(key) = CLASSES_ROOT.open(&ext) {
            if let Ok(mime_type) = key.get_value("Content Type") {
                let mime = mime_type.try_into()?;
                table.entry(mime).or_default().push(ext);
            } else if let Ok(mime_type) = key.get_value("ContentType") {
                let mime = mime_type.try_into()?;
                table.entry(mime).or_default().push(ext);
            }
        }
    };

    Ok(table)
}