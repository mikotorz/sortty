use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{path}: {message}", path = .path.display())]
    Io { path: PathBuf, message: String },
    #[error("path does not exist: {}", .0.display())]
    NotFound(PathBuf),
    #[error("path is not a directory: {}", .0.display())]
    NotADirectory(PathBuf),
    #[error(
        "This looks like a system folder ({}) — sortty won't scan drive roots or core Windows folders. Pick a more specific folder.",
        .0.display()
    )]
    ProtectedPath(PathBuf),
    #[error("config error: {0}")]
    Config(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("plan and selection do not match: {0}")]
    InvalidPlan(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        AppError::Io {
            path: path.into(),
            message: describe_io_error(&source),
        }
    }
}

/// Turns a raw OS error into a plain-English reason where we recognize it,
/// falling back to the OS-provided message otherwise.
fn describe_io_error(e: &std::io::Error) -> String {
    match e.raw_os_error() {
        // ERROR_SHARING_VIOLATION
        Some(32) => "this file is open in another program".to_string(),
        // ERROR_ACCESS_DENIED
        Some(5) => "permission denied — check the file isn't read-only, or try running as administrator".to_string(),
        _ => match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                "permission denied — check the file isn't read-only, or try running as administrator".to_string()
            }
            std::io::ErrorKind::NotFound => {
                "no longer exists (it may have already been moved or deleted)".to_string()
            }
            _ => e.to_string(),
        },
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
