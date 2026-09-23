use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::settings::{self, AppSettings};
use crate::domain::category::CategoryRules;
use crate::error::AppError;

fn config_dir(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
pub async fn get_category_rules(app: AppHandle) -> Result<CategoryRules, AppError> {
    settings::load_category_rules(&config_dir(&app)?)
}

#[tauri::command]
pub async fn save_category_rules(app: AppHandle, rules: CategoryRules) -> Result<(), AppError> {
    settings::save_category_rules(&config_dir(&app)?, &rules)
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<AppSettings, AppError> {
    settings::load_settings(&config_dir(&app)?)
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings_value: AppSettings) -> Result<(), AppError> {
    settings::save_settings(&config_dir(&app)?, &settings_value)
}

#[tauri::command]
pub async fn open_config_folder(app: AppHandle) -> Result<(), AppError> {
    let dir = config_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| AppError::io(dir.clone(), e))?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::Other(e.to_string()))
}
