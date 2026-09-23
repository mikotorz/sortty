use crate::error::AppError;

const ILLEGAL_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Validates a name meant to be used as a single path segment joined onto a
/// scan root — a category name, or the trash/archive folder name. These are
/// user-editable (via Settings, or by hand-editing the TOML config), and are
/// joined onto `root` with no further checks by the engine, so this is what
/// stands between a value like `"../../Desktop"` and a move landing outside
/// the scanned root. Rejects anything that isn't a single, ordinary folder
/// name: empty names, `.`/`..`, path separators, Windows-illegal characters,
/// and Windows-reserved device names.
pub fn validate_folder_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Config("folder name can't be empty".to_string()));
    }
    if name == "." || name == ".." {
        return Err(AppError::Config(format!(
            "\"{name}\" isn't a valid folder name"
        )));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(AppError::Config(format!(
            "\"{name}\" can't contain a path separator"
        )));
    }
    if name
        .chars()
        .any(|c| ILLEGAL_CHARS.contains(&c) || c.is_control())
    {
        return Err(AppError::Config(format!(
            "\"{name}\" contains a character that isn't allowed in a folder name"
        )));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(AppError::Config(format!(
            "\"{name}\" can't end with a space or a period"
        )));
    }
    let base = name.split('.').next().unwrap_or(name);
    if WINDOWS_RESERVED_NAMES.contains(&base.to_lowercase().as_str()) {
        return Err(AppError::Config(format!(
            "\"{name}\" is a reserved name on Windows"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_names() {
        assert!(validate_folder_name("Images").is_ok());
        assert!(validate_folder_name(".sortty-trash").is_ok());
        assert!(validate_folder_name("My Documents").is_ok());
    }

    #[test]
    fn rejects_path_traversal_and_separators() {
        assert!(validate_folder_name("..").is_err());
        assert!(validate_folder_name(".").is_err());
        assert!(validate_folder_name("../../Desktop").is_err());
        assert!(validate_folder_name("a/b").is_err());
        assert!(validate_folder_name("a\\b").is_err());
    }

    #[test]
    fn rejects_empty_and_illegal_characters() {
        assert!(validate_folder_name("").is_err());
        assert!(validate_folder_name("   ").is_err());
        assert!(validate_folder_name("bad:name").is_err());
        assert!(validate_folder_name("bad*name").is_err());
        assert!(validate_folder_name("trailing.").is_err());
        assert!(validate_folder_name("trailing ").is_err());
    }

    #[test]
    fn rejects_windows_reserved_device_names() {
        assert!(validate_folder_name("CON").is_err());
        assert!(validate_folder_name("nul").is_err());
        assert!(validate_folder_name("com1").is_err());
        assert!(validate_folder_name("com1.txt").is_err());
    }
}
