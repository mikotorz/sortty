use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

use crate::error::AppError;

const MAX_PREVIEW_BYTES: u64 = 15 * 1024 * 1024;

fn mime_for_extension(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// Reads a small image file and returns it as a data URL, for thumbnail
/// previews in the plan-preview grid. Only known image extensions under
/// a size cap are supported; anything else errors so the frontend falls
/// back to a generic file-type icon.
pub fn read_file_preview_at(p: &Path) -> Result<String, AppError> {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime = mime_for_extension(ext)
        .ok_or_else(|| AppError::Other("unsupported preview type".to_string()))?;

    let metadata = std::fs::metadata(p).map_err(|e| AppError::io(p.to_path_buf(), e))?;
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(AppError::Other("file too large to preview".to_string()));
    }

    let bytes = std::fs::read(p).map_err(|e| AppError::io(p.to_path_buf(), e))?;
    let encoded = STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

#[tauri::command]
pub async fn read_file_preview(path: String) -> Result<String, AppError> {
    read_file_preview_at(Path::new(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn returns_a_base64_data_url_for_a_small_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("thumb.png");
        fs::write(&path, b"not-really-a-png-but-thats-fine-here").unwrap();

        let url = read_file_preview_at(&path).unwrap();

        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn errors_for_unsupported_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        fs::write(&path, b"hello").unwrap();

        assert!(read_file_preview_at(&path).is_err());
    }

    #[test]
    fn errors_for_a_file_over_the_size_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.png");
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_PREVIEW_BYTES + 1).unwrap();

        let err = read_file_preview_at(&path).unwrap_err();
        assert!(matches!(err, AppError::Other(_)));
    }

    #[test]
    fn errors_for_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.png");

        assert!(read_file_preview_at(&path).is_err());
    }
}
