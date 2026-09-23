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
#[tauri::command]
pub async fn read_file_preview(path: String) -> Result<String, AppError> {
    let p = Path::new(&path);
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mime = mime_for_extension(ext).ok_or_else(|| AppError::Other("unsupported preview type".to_string()))?;

    let metadata = std::fs::metadata(p).map_err(|e| AppError::io(p.to_path_buf(), e))?;
    if metadata.len() > MAX_PREVIEW_BYTES {
        return Err(AppError::Other("file too large to preview".to_string()));
    }

    let bytes = std::fs::read(p).map_err(|e| AppError::io(p.to_path_buf(), e))?;
    let encoded = STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}
