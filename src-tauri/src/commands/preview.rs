use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;
use std::path::Path;

use crate::commands::blocking;
use crate::error::AppError;

/// Longest edge, in pixels, of the thumbnails sent to the preview grid —
/// the grid's largest tile is well under this.
const THUMBNAIL_EDGE: u32 = 256;

/// Raster images bigger than this aren't decoded for a thumbnail at all.
/// Decoding is what costs memory, not the file size, but this keeps an
/// oversized scan or a huge TIFF-in-disguise from stalling the grid.
const MAX_RASTER_BYTES: u64 = 50 * 1024 * 1024;

/// SVGs are sent as-is (they're text and scale for free), so they get a
/// much smaller cap.
const MAX_SVG_BYTES: u64 = 2 * 1024 * 1024;

fn is_raster_extension(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"
    )
}

/// A small thumbnail of an image file, as a data URL for the plan-preview
/// grid. Raster images are decoded and scaled down to at most
/// `THUMBNAIL_EDGE` pixels in Rust, so the window never receives the
/// original, which could be many megabytes. The thumbnail is PNG if the
/// image has transparency, JPEG otherwise. SVGs are passed through. Anything
/// else errors, and the frontend falls back to a file-type icon.
pub fn read_file_preview_at(p: &Path) -> Result<String, AppError> {
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let is_svg = ext.eq_ignore_ascii_case("svg");
    if !is_svg && !is_raster_extension(ext) {
        return Err(AppError::Other("unsupported preview type".to_string()));
    }

    let metadata = std::fs::metadata(p).map_err(|e| AppError::io(p.to_path_buf(), e))?;
    let cap = if is_svg {
        MAX_SVG_BYTES
    } else {
        MAX_RASTER_BYTES
    };
    if metadata.len() > cap {
        return Err(AppError::Other("file too large to preview".to_string()));
    }

    if is_svg {
        let bytes = std::fs::read(p).map_err(|e| AppError::io(p.to_path_buf(), e))?;
        return Ok(format!(
            "data:image/svg+xml;base64,{}",
            STANDARD.encode(bytes)
        ));
    }

    let decode_error = |e: image::ImageError| AppError::Other(format!("can't preview image: {e}"));
    let mut image = ImageReader::open(p)
        .map_err(|e| AppError::io(p.to_path_buf(), e))?
        .with_guessed_format()
        .map_err(|e| AppError::io(p.to_path_buf(), e))?
        .decode()
        .map_err(decode_error)?;
    // `thumbnail` also scales small images *up*; only ever shrink.
    if image.width() > THUMBNAIL_EDGE || image.height() > THUMBNAIL_EDGE {
        image = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE);
    }

    let (format, mime) = if image.color().has_alpha() {
        (ImageFormat::Png, "image/png")
    } else {
        (ImageFormat::Jpeg, "image/jpeg")
    };
    let mut encoded = Cursor::new(Vec::new());
    let image = if format == ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(image.to_rgb8())
    } else {
        image
    };
    image.write_to(&mut encoded, format).map_err(decode_error)?;
    Ok(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(encoded.into_inner())
    ))
}

#[tauri::command]
pub async fn read_file_preview(path: String) -> Result<String, AppError> {
    blocking(move || read_file_preview_at(Path::new(&path))).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage, RgbaImage};
    use std::fs;

    fn decoded(url: &str) -> DynamicImage {
        let b64 = url.split_once(',').unwrap().1;
        image::load_from_memory(&STANDARD.decode(b64).unwrap()).unwrap()
    }

    #[test]
    fn scales_a_large_image_down_to_a_small_jpeg_thumbnail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.png");
        RgbImage::new(2000, 1000).save(&path).unwrap();

        let url = read_file_preview_at(&path).unwrap();

        assert!(url.starts_with("data:image/jpeg;base64,"));
        let thumb = decoded(&url);
        assert_eq!((thumb.width(), thumb.height()), (256, 128));
    }

    #[test]
    fn keeps_transparency_as_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("icon.png");
        RgbaImage::new(64, 64).save(&path).unwrap();

        let url = read_file_preview_at(&path).unwrap();

        assert!(url.starts_with("data:image/png;base64,"));
        assert_eq!(decoded(&url).width(), 64);
    }

    #[test]
    fn passes_svg_through() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logo.svg");
        fs::write(&path, b"<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap();

        assert!(read_file_preview_at(&path)
            .unwrap()
            .starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn errors_for_an_undecodable_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.png");
        fs::write(&path, b"not-really-a-png").unwrap();

        assert!(read_file_preview_at(&path).is_err());
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
        file.set_len(MAX_RASTER_BYTES + 1).unwrap();

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
