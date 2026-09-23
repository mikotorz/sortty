use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// How much of each file the first, cheap pass hashes. Most same-size files
/// that aren't duplicates already differ in their first 64 KiB, so only the
/// ones that match here get a full (possibly multi-gigabyte) hash.
const PARTIAL_BYTES: u64 = 64 * 1024;

/// Which hashing pass a progress report is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashStage {
    /// Hashing the first `PARTIAL_BYTES` of every same-size candidate.
    Checking,
    /// Fully hashing the files whose first `PARTIAL_BYTES` matched.
    Comparing,
}

/// Hashes a file (or its first `limit` bytes) in fixed-size chunks rather
/// than reading it whole into memory, so a large duplicate candidate (video,
/// VM image, ISO) doesn't cause memory pressure.
fn hash_file(path: &Path, limit: Option<u64>) -> Result<String, AppError> {
    let file = std::fs::File::open(path).map_err(|e| AppError::io(path.to_path_buf(), e))?;
    let reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    match limit {
        Some(limit) => std::io::copy(&mut reader.take(limit), &mut hasher),
        None => std::io::copy(&mut { reader }, &mut hasher),
    }
    .map_err(|e| AppError::io(path.to_path_buf(), e))?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Hashes `files` in parallel, reporting `(stage, done, total)` as each one
/// finishes. One unreadable file (locked, permission-denied, a cloud-sync
/// placeholder) is skipped rather than failing the whole plan — the same
/// policy the scanner uses. Once `should_cancel` fires, the remaining files
/// are skipped without being read.
fn hash_all<'a>(
    files: &[&'a FileEntry],
    limit: Option<u64>,
    stage: HashStage,
    on_progress: &(dyn Fn(HashStage, usize, usize) + Sync),
    should_cancel: &(dyn Fn() -> bool + Sync),
) -> Vec<(String, &'a FileEntry)> {
    let total = files.len();
    let done = AtomicUsize::new(0);
    files
        .par_iter()
        .filter_map(|e| {
            if should_cancel() {
                return None;
            }
            let hashed = match hash_file(&e.path, limit) {
                Ok(h) => Some((h, *e)),
                Err(err) => {
                    log::warn!(
                        "sortty: skipping {} in duplicate check: {err}",
                        e.path.display()
                    );
                    None
                }
            };
            on_progress(stage, done.fetch_add(1, Ordering::Relaxed) + 1, total);
            hashed
        })
        .collect()
}

/// Groups hashed files by (size, hash), keeping only groups of two or more.
fn matching_groups(hashed: Vec<(String, &FileEntry)>) -> Vec<Vec<&FileEntry>> {
    let mut groups: HashMap<(u64, String), Vec<&FileEntry>> = HashMap::new();
    for (hash, entry) in hashed {
        groups
            .entry((entry.size_bytes, hash))
            .or_default()
            .push(entry);
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
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

pub fn build_plan(root: &Path, entries: &[FileEntry], options: &DedupOptions) -> Plan {
    build_plan_with_progress(root, entries, options, &|_, _, _| {}, &|| false)
        .expect("never cancelled")
}

/// Finds files with identical contents in three narrowing passes: same
/// size, then same first 64 KiB, then (only for files bigger than that) same
/// full hash. Returns `None` if `should_cancel` fired during hashing.
pub fn build_plan_with_progress(
    root: &Path,
    entries: &[FileEntry],
    options: &DedupOptions,
    on_progress: &(dyn Fn(HashStage, usize, usize) + Sync),
    should_cancel: &(dyn Fn() -> bool + Sync),
) -> Option<Plan> {
    let mut by_size: HashMap<u64, Vec<&FileEntry>> = HashMap::new();
    for entry in entries {
        if entry.size_bytes >= options.min_size_bytes && entry.size_bytes > 0 {
            by_size.entry(entry.size_bytes).or_default().push(entry);
        }
    }
    let candidates: Vec<&FileEntry> = by_size
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .collect();

    let partial = hash_all(
        &candidates,
        Some(PARTIAL_BYTES),
        HashStage::Checking,
        on_progress,
        should_cancel,
    );
    if should_cancel() {
        return None;
    }

    // A file no bigger than PARTIAL_BYTES was hashed in full already.
    let (mut duplicate_groups, need_full): (Vec<_>, Vec<_>) = matching_groups(partial)
        .into_iter()
        .partition(|group| group[0].size_bytes <= PARTIAL_BYTES);
    let need_full: Vec<&FileEntry> = need_full.into_iter().flatten().collect();
    let full = hash_all(
        &need_full,
        None,
        HashStage::Comparing,
        on_progress,
        should_cancel,
    );
    if should_cancel() {
        return None;
    }
    duplicate_groups.extend(matching_groups(full));

    let mut operations = Vec::new();
    for dup_group in duplicate_groups {
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

    // HashMap iteration order is random; sort so the same folder always
    // produces the same plan order.
    operations.sort_by(|a, b| a.source.cmp(&b.source));
    Some(Plan::new(root.to_path_buf(), PlanMode::Dedup, operations))
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
        );

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
        );

        assert_eq!(plan.operations.len(), 1);
    }

    fn options() -> DedupOptions {
        DedupOptions {
            min_size_bytes: 1,
            keep_strategy: KeepStrategy::ShortestPath,
            trash_folder_name: ".sortty-trash".to_string(),
        }
    }

    #[test]
    fn large_files_matching_only_in_their_first_64k_are_not_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let size = (PARTIAL_BYTES * 2) as usize;
        let mut same_prefix = vec![7u8; size];
        fs::write(root.join("a.bin"), &same_prefix).unwrap();
        fs::write(root.join("b.bin"), &same_prefix).unwrap();
        *same_prefix.last_mut().unwrap() = 8;
        fs::write(root.join("c.bin"), &same_prefix).unwrap();
        let entries = vec![
            entry_for(root.join("a.bin"), size as u64),
            entry_for(root.join("b.bin"), size as u64),
            entry_for(root.join("c.bin"), size as u64),
        ];

        let plan = build_plan(root, &entries, &options());

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].source, root.join("b.bin"));
    }

    #[test]
    fn reports_both_hashing_stages() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let size = (PARTIAL_BYTES + 1) as usize;
        fs::write(root.join("a.bin"), vec![1u8; size]).unwrap();
        fs::write(root.join("b.bin"), vec![1u8; size]).unwrap();
        let entries = vec![
            entry_for(root.join("a.bin"), size as u64),
            entry_for(root.join("b.bin"), size as u64),
        ];
        let seen = std::sync::Mutex::new(Vec::new());

        build_plan_with_progress(
            root,
            &entries,
            &options(),
            &|stage, done, total| seen.lock().unwrap().push((stage, done, total)),
            &|| false,
        )
        .unwrap();

        let mut seen = seen.into_inner().unwrap();
        seen.sort_by_key(|&(stage, done, _)| (stage == HashStage::Comparing, done));
        assert_eq!(
            seen,
            vec![
                (HashStage::Checking, 1, 2),
                (HashStage::Checking, 2, 2),
                (HashStage::Comparing, 1, 2),
                (HashStage::Comparing, 2, 2),
            ]
        );
    }

    #[test]
    fn cancelling_returns_no_plan() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.txt"), b"same").unwrap();
        fs::write(root.join("b.txt"), b"same").unwrap();
        let entries = vec![
            entry_for(root.join("a.txt"), 4),
            entry_for(root.join("b.txt"), 4),
        ];

        let plan = build_plan_with_progress(root, &entries, &options(), &|_, _, _| {}, &|| true);

        assert!(plan.is_none());
    }
}
