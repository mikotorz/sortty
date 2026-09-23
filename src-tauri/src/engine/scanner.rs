use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::domain::entry::FileEntry;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Off by default: only loose files directly in the chosen folder are touched.
    /// Existing subfolders (installer folders, project folders, etc.) are left
    /// alone unless the user explicitly opts in.
    #[serde(default)]
    pub include_subfolders: bool,
    /// Folder names to skip anywhere in the tree (e.g. ".sortty-trash", ".sortty-archive").
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Full folder paths to skip anywhere in the tree, chosen by the user
    /// (e.g. an installer folder they don't want touched even with
    /// `include_subfolders` on). Matched by exact path, unlike `exclude`.
    #[serde(default)]
    pub exclude_folders: Vec<String>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            include_subfolders: false,
            exclude: vec![
                ".sortty-trash".to_string(),
                ".sortty-archive".to_string(),
            ],
            exclude_folders: Vec::new(),
        }
    }
}

const JUNK_NAMES: &[&str] = &["thumbs.db", "desktop.ini", ".ds_store", "ehthumbs.db"];

const INCOMPLETE_DOWNLOAD_SUFFIXES: &[&str] =
    &[".crdownload", ".part", ".partial", ".download", ".opdownload"];

/// Files that are OS/browser bookkeeping rather than something a user meant to
/// keep — never sorted, always left alone.
fn should_skip_file(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    JUNK_NAMES.contains(&lower.as_str())
        || INCOMPLETE_DOWNLOAD_SUFFIXES
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

#[cfg(windows)]
fn is_hidden_or_system(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    let attrs = metadata.file_attributes();
    attrs & FILE_ATTRIBUTE_HIDDEN != 0 || attrs & FILE_ATTRIBUTE_SYSTEM != 0
}

#[cfg(not(windows))]
fn is_hidden_or_system(_metadata: &Metadata) -> bool {
    false
}

const PROTECTED_TOP_LEVEL_NAMES: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "system volume information",
    "users",
];

/// True for a drive root (e.g. `C:\`) or a core OS folder directly under one
/// (e.g. `C:\Windows`) — places where "organizing" would be catastrophic.
fn is_protected_root(root: &Path) -> bool {
    let Some(parent) = root.parent() else {
        return true; // root has no parent: it's a drive root
    };
    if parent.parent().is_none() {
        if let Some(name) = root.file_name().map(|n| n.to_string_lossy().to_lowercase()) {
            return PROTECTED_TOP_LEVEL_NAMES.contains(&name.as_str());
        }
    }
    false
}

pub fn scan(root: &Path, options: &ScanOptions) -> Result<Vec<FileEntry>, AppError> {
    if !root.exists() {
        return Err(AppError::NotFound(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(AppError::NotADirectory(root.to_path_buf()));
    }
    if is_protected_root(root) {
        return Err(AppError::ProtectedPath(root.to_path_buf()));
    }

    let max_depth = if options.include_subfolders {
        usize::MAX
    } else {
        1
    };

    let exclude = options.exclude.clone();
    let exclude_folders: Vec<PathBuf> = options.exclude_folders.iter().map(PathBuf::from).collect();
    let walker = WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            if exclude.iter().any(|ex| ex.as_str() == name) {
                return false;
            }
            !exclude_folders.iter().any(|ex| ex == e.path())
        });

    let mut entries = Vec::new();
    for item in walker {
        let item = item.map_err(|e| AppError::Other(e.to_string()))?;
        if !item.file_type().is_file() {
            continue;
        }
        let path = item.path().to_path_buf();
        let metadata = item
            .metadata()
            .map_err(|e| AppError::io(path.clone(), e.into()))?;

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if should_skip_file(&file_name) || is_hidden_or_system(&metadata) {
            continue;
        }

        let modified: DateTime<Utc> = metadata
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        let created: Option<DateTime<Utc>> =
            metadata.created().ok().map(DateTime::<Utc>::from);

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());

        entries.push(FileEntry {
            path,
            file_name,
            extension,
            size_bytes: metadata.len(),
            modified,
            created,
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_flat_files_and_skips_excluded_dirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join(".sortty-trash")).unwrap();
        fs::write(dir.path().join(".sortty-trash").join("b.txt"), b"bye").unwrap();

        let entries = scan(dir.path(), &ScanOptions::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "a.txt");
    }

    #[test]
    fn errors_on_missing_root() {
        let missing = std::path::Path::new("D:\\definitely-not-a-real-path-sortty-test");
        assert!(scan(missing, &ScanOptions::default()).is_err());
    }

    #[test]
    fn default_is_not_recursive() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("loose.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("MATLAB installer")).unwrap();
        fs::write(dir.path().join("MATLAB installer").join("setup.exe"), b"installer").unwrap();

        let entries = scan(dir.path(), &ScanOptions::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "loose.txt");
    }

    #[test]
    fn include_subfolders_opts_into_recursion() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("loose.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested").join("inner.txt"), b"inner").unwrap();

        let options = ScanOptions {
            include_subfolders: true,
            ..ScanOptions::default()
        };
        let entries = scan(dir.path(), &options).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn exclude_folders_skips_by_full_path_while_scanning_siblings() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("keep")).unwrap();
        fs::write(dir.path().join("keep").join("a.txt"), b"a").unwrap();
        fs::create_dir(dir.path().join("skip")).unwrap();
        fs::write(dir.path().join("skip").join("b.txt"), b"b").unwrap();

        let options = ScanOptions {
            include_subfolders: true,
            exclude_folders: vec![dir.path().join("skip").to_string_lossy().to_string()],
            ..ScanOptions::default()
        };
        let entries = scan(dir.path(), &options).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "a.txt");
    }

    #[test]
    fn skips_incomplete_downloads_and_junk_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real.txt"), b"hello").unwrap();
        fs::write(dir.path().join("movie.mp4.crdownload"), b"partial").unwrap();
        fs::write(dir.path().join("Thumbs.db"), b"junk").unwrap();

        let entries = scan(dir.path(), &ScanOptions::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "real.txt");
    }

    #[test]
    fn rejects_drive_root_and_protected_windows_folder() {
        assert!(is_protected_root(Path::new("C:\\")));
        assert!(is_protected_root(Path::new("C:\\Windows")));
        assert!(is_protected_root(Path::new("D:\\Program Files")));
        assert!(!is_protected_root(Path::new("C:\\Users\\me\\Downloads")));
    }

    #[test]
    fn scan_returns_protected_path_error_for_drive_root() {
        let err = scan(Path::new("C:\\"), &ScanOptions::default()).unwrap_err();
        assert!(matches!(err, AppError::ProtectedPath(_)));
    }
}
