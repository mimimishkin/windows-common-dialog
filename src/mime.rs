//! Mapping file extensions to MIME content types via the Windows registry.

use windows_registry::{CLASSES_ROOT, Key};

use crate::utils::WinRes;

/// Registry value names that may carry the content type of an extension.
const CONTENT_TYPE_VALUES: [&str; 2] = ["Content Type", "ContentType"];

/// Returns the content type advertised by `key`, if any.
fn content_type(key: &Key) -> Option<String> {
    CONTENT_TYPE_VALUES
        .iter()
        .find_map(|name| key.get_string(name).ok())
}

/// Returns the content type registered for `extension` (e.g. `".txt"` → `"text/plain"`).
///
/// Matching is case-insensitive, just like the registry itself.
pub fn type_for_extension(extension: &str) -> WinRes<Option<String>> {
    Ok(CLASSES_ROOT
        .open(extension)
        .ok()
        .and_then(|key| content_type(&key)))
}

/// Returns every extension registered for `mime_type`/`mime_subtype`.
///
/// For example `("text", "plain")` yields `[".txt", ...]`. The preferred extension from the
/// `MIME\Database\Content Type\<type>/<subtype>` key, if any, comes first and is not
/// duplicated. An empty or `"*"` subtype matches every subtype.
pub fn extensions_for_type(mime_type: &str, mime_subtype: &str) -> WinRes<Vec<String>> {
    let mime_type = mime_type.to_ascii_lowercase();
    let mime_subtype = mime_subtype.to_ascii_lowercase();

    let mut extensions = Vec::new();

    // The registry records the preferred extension of well-known types.
    let preferred_key = format!("MIME\\Database\\Content Type\\{mime_type}/{mime_subtype}");
    if let Ok(parent) = CLASSES_ROOT.open(&preferred_key) {
        if let Ok(preferred) = parent.get_string("Extension") {
            extensions.push(preferred);
        }
    }

    for extension in CLASSES_ROOT.keys()? {
        if !extension.starts_with('.')
            || extensions
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&extension))
        {
            continue;
        }
        let registered_type = CLASSES_ROOT
            .open(&extension)
            .ok()
            .and_then(|key| content_type(&key));
        if registered_type.is_some_and(|value| matches_type(&value, &mime_type, &mime_subtype)) {
            extensions.push(extension);
        }
    }

    Ok(extensions)
}

/// Whether `content_type` is exactly `<mime_type>/<mime_subtype>` (case-insensitive).
fn matches_type(content_type: &str, mime_type: &str, mime_subtype: &str) -> bool {
    content_type
        .split_once('/')
        .is_some_and(|(type_, subtype)| {
            type_.eq_ignore_ascii_case(mime_type)
                && (mime_subtype == "*" || subtype.eq_ignore_ascii_case(mime_subtype))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_for_extension() {
        let res = type_for_extension(".txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), Some("text/plain".to_string()));
    }

    #[test]
    fn test_extensions_for_type() {
        let res = extensions_for_type("text", "plain");
        assert!(res.is_ok());
        assert!(!res.unwrap().is_empty());
    }
}