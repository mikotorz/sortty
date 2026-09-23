use std::path::Path;

use crate::error::AppError;

/// Writes `contents` to `path` without ever leaving a partially-written file
/// behind: writes to a sibling `<name>.tmp` file first, then renames it onto
/// `path`. The rename is a single filesystem operation, so a crash or power
/// loss mid-write can only ever leave the old `path` (untouched) or the new
/// one — never a truncated/corrupt one.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), AppError> {
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = path.with_file_name(tmp_name);

    std::fs::write(&tmp_path, contents).map_err(|e| AppError::io(tmp_path.clone(), e))?;
    std::fs::rename(&tmp_path, path).map_err(|e| AppError::io(path.to_path_buf(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");

        write_atomic(&path, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

        write_atomic(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");

        // No leftover temp file.
        assert!(!path.with_file_name("data.json.tmp").exists());
    }
}
