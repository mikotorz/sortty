use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::entry::FileEntry;
use crate::domain::plan::{Operation, OperationKind, Plan, PlanMode};
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

fn trash_destination(root: &Path, source: &Path, trash_folder_name: &str) -> PathBuf {
    let relative = source.strip_prefix(root).unwrap_or(source);
    root.join(trash_folder_name).join(relative)
}

fn hash_file(path: &Path) -> Result<String, AppError> {
    let bytes = std::fs::read(path).map_err(|e| AppError::io(path.to_path_buf(), e))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn pick_keeper<'a>(group: &[&'a FileEntry], strategy: KeepStrategy) -> &'a FileEntry {
    match strategy {
        KeepStrategy::OldestModified => group
            .iter()
            .min_by_key(|e| e.modified)
            .expect("group is non-empty"),
        KeepStrategy::ShortestPath => group
            .iter()
            .min_by_key(|e| e.path.as_os_str().len())
            .expect("group is non-empty"),
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
        let hashed: Vec<(String, &FileEntry)> = group
            .par_iter()
            .map(|e| hash_file(&e.path).map(|h| (h, *e)))
            .collect::<Result<_, _>>()?;

        let mut by_hash: HashMap<String, Vec<&FileEntry>> = HashMap::new();
        for (hash, entry) in hashed {
            by_hash.entry(hash).or_default().push(entry);
        }

        for dup_group in by_hash.into_values().filter(|g| g.len() > 1) {
            let keeper = pick_keeper(&dup_group, options.keep_strategy);
            for entry in &dup_group {
                if entry.path == keeper.path {
                    continue;
                }
                operations.push(Operation::new(
                    OperationKind::MoveToTrash,
                    entry.path.clone(),
                    trash_destination(root, &entry.path, &options.trash_folder_name),
                    format!("Duplicate of {}", keeper.path.display()),
                    entry.size_bytes,
                ));
            }
        }
    }

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
}
