use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::domain::category::CategoryRules;
use crate::domain::folder_name::validate_folder_name;
use crate::domain::plan::PlanMode;
use crate::engine::cleanup::StaleAction;
use crate::engine::dedup::KeepStrategy;
use crate::engine::sort_by_date::{DateGranularity, DateSource};
use crate::error::AppError;
use crate::fsutil::write_atomic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub default_root: Option<String>,
    pub last_used_mode: PlanMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortByDateSettings {
    pub granularity: DateGranularity,
    pub date_source: DateSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupSettings {
    pub stale_days: u32,
    pub date_source: DateSource,
    pub action: StaleAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupSettings {
    pub keep_strategy: KeepStrategy,
    pub min_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashSettings {
    pub staging_folder_name: String,
    pub archive_folder_name: String,
}

impl TrashSettings {
    /// Every staging folder name a scan must never descend into: the
    /// configured trash/archive names plus the built-in defaults, so files
    /// already staged under a folder's old name stay protected after the
    /// user renames it in Settings.
    pub fn staging_folder_names(&self) -> Vec<String> {
        let mut names = vec![
            self.staging_folder_name.clone(),
            self.archive_folder_name.clone(),
        ];
        for default in [DEFAULT_TRASH_FOLDER, DEFAULT_ARCHIVE_FOLDER] {
            if !names.iter().any(|n| n == default) {
                names.push(default.to_string());
            }
        }
        names
    }
}

pub const DEFAULT_TRASH_FOLDER: &str = ".sortty-trash";
pub const DEFAULT_ARCHIVE_FOLDER: &str = ".sortty-archive";

/// How long History keeps runs (ADR 0020). `keep_days == 0` keeps them forever.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HistorySettings {
    pub keep_days: u32,
}

pub const DEFAULT_HISTORY_KEEP_DAYS: u32 = 90;
pub const MAX_HISTORY_KEEP_DAYS: u32 = 3650;

impl Default for HistorySettings {
    fn default() -> Self {
        HistorySettings {
            keep_days: DEFAULT_HISTORY_KEEP_DAYS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub sort_by_date: SortByDateSettings,
    pub cleanup: CleanupSettings,
    pub dedup: DedupSettings,
    pub trash: TrashSettings,
    /// Added after the first release, so older settings.toml files without a
    /// `[history]` table still load.
    #[serde(default)]
    pub history: HistorySettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            general: GeneralSettings {
                default_root: None,
                last_used_mode: PlanMode::SortByType,
            },
            sort_by_date: SortByDateSettings {
                granularity: DateGranularity::YearMonth,
                date_source: DateSource::Modified,
            },
            cleanup: CleanupSettings {
                stale_days: 180,
                date_source: DateSource::Modified,
                action: StaleAction::Archive,
            },
            dedup: DedupSettings {
                keep_strategy: KeepStrategy::OldestModified,
                min_size_bytes: 1024,
            },
            trash: TrashSettings {
                staging_folder_name: DEFAULT_TRASH_FOLDER.to_string(),
                archive_folder_name: DEFAULT_ARCHIVE_FOLDER.to_string(),
            },
            history: HistorySettings::default(),
        }
    }
}

fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.toml")
}

fn categories_path(config_dir: &Path) -> PathBuf {
    config_dir.join("categories.toml")
}

pub fn load_settings(config_dir: &Path) -> Result<AppSettings, AppError> {
    let path = settings_path(config_dir);
    if !path.exists() {
        let defaults = AppSettings::default();
        save_settings(config_dir, &defaults)?;
        return Ok(defaults);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| AppError::io(path.clone(), e))?;
    toml::from_str(&text).map_err(|e| AppError::Config(format!("settings.toml: {e}")))
}

pub fn save_settings(config_dir: &Path, settings: &AppSettings) -> Result<(), AppError> {
    validate_folder_name(&settings.trash.staging_folder_name)?;
    validate_folder_name(&settings.trash.archive_folder_name)?;
    if settings.history.keep_days > MAX_HISTORY_KEEP_DAYS {
        return Err(AppError::Config(format!(
            "history can be kept for at most {MAX_HISTORY_KEEP_DAYS} days (use 0 to keep it forever)"
        )));
    }

    std::fs::create_dir_all(config_dir).map_err(|e| AppError::io(config_dir.to_path_buf(), e))?;
    let path = settings_path(config_dir);
    let text = toml::to_string_pretty(settings)
        .map_err(|e| AppError::Config(format!("failed to serialize settings: {e}")))?;
    write_atomic(&path, &text)
}

pub fn load_category_rules(config_dir: &Path) -> Result<CategoryRules, AppError> {
    let path = categories_path(config_dir);
    if !path.exists() {
        let defaults = CategoryRules::default_rules();
        save_category_rules(config_dir, &defaults)?;
        return Ok(defaults);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| AppError::io(path.clone(), e))?;
    toml::from_str(&text).map_err(|e| AppError::Config(format!("categories.toml: {e}")))
}

pub fn save_category_rules(config_dir: &Path, rules: &CategoryRules) -> Result<(), AppError> {
    validate_folder_name(&rules.other_folder_name)?;
    for name in rules.categories.keys() {
        validate_folder_name(name)?;
    }

    std::fs::create_dir_all(config_dir).map_err(|e| AppError::io(config_dir.to_path_buf(), e))?;
    let path = categories_path(config_dir);
    let text = toml::to_string_pretty(rules)
        .map_err(|e| AppError::Config(format!("failed to serialize categories: {e}")))?;
    write_atomic(&path, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_defaults_on_first_load_and_reloads_same_values() {
        let dir = tempfile::tempdir().unwrap();
        let settings = load_settings(dir.path()).unwrap();
        assert_eq!(settings.cleanup.stale_days, 180);

        let reloaded = load_settings(dir.path()).unwrap();
        assert_eq!(reloaded.cleanup.stale_days, settings.cleanup.stale_days);
    }

    #[test]
    fn settings_without_a_history_table_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut text = toml::to_string_pretty(&AppSettings::default()).unwrap();
        let cut = text.find("[history]").unwrap();
        text.truncate(cut);
        std::fs::write(dir.path().join("settings.toml"), text).unwrap();

        let settings = load_settings(dir.path()).unwrap();
        assert_eq!(settings.history.keep_days, DEFAULT_HISTORY_KEEP_DAYS);
    }

    #[test]
    fn rejects_an_out_of_range_history_setting() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = AppSettings::default();
        settings.history.keep_days = MAX_HISTORY_KEEP_DAYS + 1;
        assert!(save_settings(dir.path(), &settings).is_err());
        settings.history.keep_days = 0;
        assert!(save_settings(dir.path(), &settings).is_ok());
    }

    #[test]
    fn round_trips_category_rules() {
        let dir = tempfile::tempdir().unwrap();
        let rules = load_category_rules(dir.path()).unwrap();
        assert!(rules.categories.contains_key("Images"));
    }

    #[test]
    fn rejects_path_traversal_in_trash_folder_names() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = AppSettings::default();
        settings.trash.staging_folder_name = "../../Desktop".to_string();
        assert!(save_settings(dir.path(), &settings).is_err());
    }

    #[test]
    fn rejects_path_traversal_in_category_names() {
        let dir = tempfile::tempdir().unwrap();
        let mut rules = CategoryRules::default_rules();
        rules
            .categories
            .insert("../escape".to_string(), rules.categories["Images"].clone());
        assert!(save_category_rules(dir.path(), &rules).is_err());
    }
}
