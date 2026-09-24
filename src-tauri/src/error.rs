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
        "This looks like a system or app-data folder ({}) — sortty won't scan drive roots, core Windows folders, or folders where programs keep their data. Pick a more specific folder.",
        .0.display()
    )]
    ProtectedPath(PathBuf),
    #[error(
        "sortty won't reach into every subfolder of your whole user folder ({}) — that would include app data and settings. Turn off \"Include files in subfolders too\", or pick a more specific folder like Downloads.",
        .0.display()
    )]
    ProtectedRecursive(PathBuf),
    #[error("config error: {0}")]
    Config(String),
    #[error("run not found: {0}")]
    RunNotFound(String),
    #[error("plan and selection do not match: {0}")]
    InvalidPlan(String),
    #[error("run {0} was already undone")]
    AlreadyUndone(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Stable, machine-readable name of the variant, sent to the frontend
    /// alongside the message so it can react to *what* went wrong (ADR 0019).
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Io { .. } => "io",
            AppError::NotFound(_) => "not_found",
            AppError::NotADirectory(_) => "not_a_directory",
            AppError::ProtectedPath(_) => "protected_path",
            AppError::ProtectedRecursive(_) => "protected_recursive",
            AppError::Config(_) => "config",
            AppError::RunNotFound(_) => "run_not_found",
            AppError::InvalidPlan(_) => "invalid_plan",
            AppError::AlreadyUndone(_) => "already_undone",
            AppError::Other(_) => "other",
        }
    }

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
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_kind_and_message() {
        let cases = [
            (AppError::io("C:/a", std::io::Error::other("boom")), "io"),
            (AppError::NotFound("C:/a".into()), "not_found"),
            (AppError::NotADirectory("C:/a".into()), "not_a_directory"),
            (
                AppError::ProtectedPath("C:/Windows".into()),
                "protected_path",
            ),
            (
                AppError::ProtectedRecursive("C:/Users/me".into()),
                "protected_recursive",
            ),
            (AppError::Config("bad".into()), "config"),
            (AppError::RunNotFound("r1".into()), "run_not_found"),
            (AppError::InvalidPlan("stale".into()), "invalid_plan"),
            (AppError::AlreadyUndone("r1".into()), "already_undone"),
            (AppError::Other("x".into()), "other"),
        ];
        for (err, kind) in cases {
            let json = serde_json::to_value(&err).unwrap();
            assert_eq!(
                json,
                serde_json::json!({ "kind": kind, "message": err.to_string() })
            );
        }
    }
}
