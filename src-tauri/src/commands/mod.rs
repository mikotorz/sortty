pub mod apply;
pub mod browse;
pub mod cancel;
pub mod config;
pub mod history;
pub mod plan;
pub mod preview;
pub mod scan;
pub mod trash;

use crate::error::AppError;

/// Runs `work` on Tauri's blocking thread pool and awaits it. Commands that
/// walk folders, hash, or move files use this so the filesystem work never
/// ties up the async runtime's worker threads, which also serve every other
/// IPC call the window makes meanwhile.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| AppError::Other(format!("background task failed: {e}")))?
}
