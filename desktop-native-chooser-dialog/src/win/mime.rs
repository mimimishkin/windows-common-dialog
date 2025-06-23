use crate::win::utils::WinRes;
use windows_registry::CLASSES_ROOT;

pub fn type_for_extension(extension: &str) -> WinRes<Option<String>> {
    for ext in CLASSES_ROOT.keys()? {
        if ext == extension {
            if let Ok(key) = CLASSES_ROOT.open(&ext) {
                if let Ok(content_type) = key.get_string("Content Type") {
                    return Ok(Some(content_type));
                } else if let Ok(content_type) = key.get_string("ContentType") {
                    return Ok(Some(content_type));
                }
            }
        }
    };

    Ok(None)
}

pub fn extensions_for_type(mime_type: &str, mime_subtype: &str) -> WinRes<Vec<String>> {
    let mut extensions = vec![];
    
    if let Ok(preferred) = CLASSES_ROOT
        .open(format!("MIME\\Database\\Content Type\\{mime_type}/{mime_subtype}"))
        .and_then(|k| k.get_string("Extension")) 
    {
        extensions.push(preferred)
    }
    
    for ext in CLASSES_ROOT.keys()? {
        if !ext.starts_with('.') || extensions.first().is_none_or(|s| s == &ext) { 
            continue
        }
        
        if let Ok(key) = CLASSES_ROOT.open(&ext) {
            if let Ok(content_type) = key.get_string("Content Type") {
                if content_type.starts_with(mime_type) && (mime_subtype == "*" || content_type.ends_with(mime_subtype)) {
                    extensions.push(ext);
                }
            } else if let Ok(content_type) = key.get_string("ContentType") {
                if content_type.starts_with(mime_type) && (mime_subtype == "*" || content_type.ends_with(mime_subtype)) {
                    extensions.push(ext);
                }
            }
        }
    };

    Ok(extensions)
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
        println!("{res:?}");
        assert!(res.is_ok());
        assert!(!res.unwrap().is_empty());
    }
}