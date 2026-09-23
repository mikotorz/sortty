use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

use crate::domain::entry::FileEntry;
use crate::domain::plan::{staged_destination, Operation, OperationKind, Plan, PlanMode};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeepStrategy {
    /// Keep the file with the oldest modified date; move newer copies to trash.
    OldestModified,
    /// Keep the file with the shortest path; move the rest to trash.
    ShortestPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupOptions {
    pub min_size_bytes: u64,
    pub keep_strategy: KeepStrategy,
    pub trash_folder_name: String,
}

/// Hashes a file in fixed-size chunks rather than reading it whole into
/// memory, so a large duplicate candidate (video, VM image, ISO) doesn't
/// cause memory pressure.
fn hash_file(path: &Path) -> Result<String, AppError> {
    let file = std::fs::File::open(path).map_err(|e| AppError::io(path.to_path_buf(), e))?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut reader, &mut hasher).map_err(|e| AppError::io(path.to_path_buf(), e))?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Returns `None` only if `group` is empty, which never happens for a real
/// duplicate group (callers only pass groups already filtered to `len() > 1`).
fn pick_keeper<'a>(group: &[&'a FileEntry], strategy: KeepStrategy) -> Option<&'a FileEntry> {
    match strategy {
        KeepStrategy::OldestModified => group.iter().min_by_key(|e| e.modified).copied(),
        KeepStrategy::ShortestPath => group
            .iter()
            .min_by_key(|e| e.path.as_os_str().len())
            .copied(),
    }
}

pub fn build_plan(
    root: &Path,
    entries: &[FileEntry],
    options: &DedupOptions,
) -> Result<Plan, AppError> {
    let mut by_size: HashMap<u64, Vec<&FileEntry>> = HashMap::new();
    for entry in entries {
        if entry.size_bytes >= options.min_size_bytes && entry.size_bytes > 0 {
            by_size.entry(entry.size_bytes).or_default().push(entry);
        }
    }

    let candidate_groups: Vec<Vec<&FileEntry>> = by_size
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();

    let mut operations = Vec::new();

    for group in candidate_groups {
        // One unreadable file (locked, permission-denied, a cloud-sync
        // placeholder) is skipped, not allowed to fail the whole plan —
        // the same policy the scanner uses.
        let hashed: Vec<(String, &FileEntry)> = group
            .par_iter()
            .filter_map(|e| match hash_file(&e.path) {
                Ok(h) => Some((h, *e)),
                Err(err) => {
                    log::warn!(
                        "sortty: skipping {} in duplicate check: {err}",
                        e.path.display()
                    );
                    None
                }
            })
            .collect();

        let mut by_hash: HashMap<String, Vec<&FileEntry>> = HashMap::new();
        for (hash, entry) in hashed {
            by_hash.entry(hash).or_default().push(entry);
        }

        for dup_group in by_hash.into_values().filter(|g| g.len() > 1) {
            let Some(keeper) = pick_keeper(&dup_group, options.keep_strategy) else {
                continue;
            };
            for entry in &dup_group {
                if entry.path == keeper.path {
                    continue;
                }
                operations.push(Operation::new(
                    OperationKind::MoveToTrash,
                    entry.path.clone(),
                    staged_destination(root, &entry.path, &options.trash_folder_name),
                    format!("Duplicate of {}", keeper.path.display()),
                    entry.size_bytes,
                ));
            }
        }
    }

    // HashMap iteration order is random; sort so the same folder always
    // produces the same plan order.
    operations.sort_by(|a, b| a.source.cmp(&b.source));
    Ok(Plan::new(root.to_path_buf(), PlanMode::Dedup, operations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::fs;

    #[test]
    fn keeps_oldest_and_trashes_true_duplicates_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"same content").unwrap();
        fs::write(root.join("b.txt"), b"same content").unwrap();
        fs::write(root.join("c.txt"), b"different!!!").unwrap(); // same size, different content

        let older = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        let entries = vec![
            FileEntry {
                path: root.join("a.txt"),
                file_name: "a.txt".into(),
                extension: Some("txt".into()),
                size_bytes: 12,
                modified: newer,
                created: None,
            },
            FileEntry {
                path: root.join("b.txt"),
                file_name: "b.txt".into(),
                extension: Some("txt".into()),
                size_bytes: 12,
                modified: older,
                created: None,
            },
            FileEntry {
                path: root.join("c.txt"),
                file_name: "c.txt".into(),
                extension: Some("txt".into()),
                size_bytes: 12,
                modified: newer,
                created: None,
            },
        ];

        let plan = build_plan(
            root,
            &entries,
            &DedupOptions {
                min_size_bytes: 1,
                keep_strategy: KeepStrategy::OldestModified,
                trash_folder_name: ".sortty-trash".to_string(),
            },
        )
        .unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].source, root.join("a.txt"));
    }

    fn entry_for(path: std::path::PathBuf, size: u64) -> FileEntry {
        FileEntry {
            file_name: path.file_name().unwrap().to_string_lossy().to_string(),
            path,
            extension: Some("txt".into()),
            size_bytes: size,
            modified: Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            created: None,
        }
    }

    /// Regression: one file that couldn't be read (here, one that vanished
    /// after the scan) used to fail the whole duplicate check.
    #[test]
    fn an_unreadable_candidate_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"same content").unwrap();
        fs::write(root.join("b.txt"), b"same content").unwrap();
        let entries = vec![
            entry_for(root.join("a.txt"), 12),
            entry_for(root.join("b.txt"), 12),
            entry_for(root.join("gone.txt"), 12),
        ];

        let plan = build_plan(
            root,
            &entries,
            &DedupOptions {
                min_size_bytes: 1,
                keep_strategy: KeepStrategy::ShortestPath,
                trash_folder_name: ".sortty-trash".to_string(),
            },
        )
        .unwrap();

        assert_eq!(plan.operations.len(), 1);
    }
}
