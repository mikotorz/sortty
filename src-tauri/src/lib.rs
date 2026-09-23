pub mod apply;
pub mod commands;
pub mod config;
pub mod domain;
pub mod engine;
pub mod error;
pub mod fsutil;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::cancel::CancelFlag::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan::scan_folder,
            commands::plan::generate_plan,
            commands::apply::apply_plan,
            commands::cancel::cancel_current_operation,
            commands::history::list_runs,
            commands::history::get_run,
            commands::history::undo_run,
            commands::history::undo_last_run,
            commands::config::get_category_rules,
            commands::config::save_category_rules,
            commands::config::get_settings,
            commands::config::save_settings,
            commands::config::reset_settings,
            commands::config::reset_category_rules,
            commands::config::open_config_folder,
            commands::preview::read_file_preview,
            commands::browse::browse_folder,
            commands::browse::delete_files,
            commands::trash::preview_staging_folder,
            commands::trash::empty_staging_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
