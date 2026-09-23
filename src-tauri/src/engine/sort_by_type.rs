use std::path::Path;

use crate::domain::category::CategoryRules;
use crate::domain::entry::FileEntry;
use crate::domain::plan::{Operation, OperationKind, Plan, PlanMode};

pub fn build_plan(root: &Path, entries: &[FileEntry], rules: &CategoryRules) -> Plan {
    let mut operations = Vec::new();

    for entry in entries {
        let category = rules.categorize(entry.extension.as_deref());
        let dest_dir = root.join(&category);
        let destination = dest_dir.join(&entry.file_name);

        // Already anywhere under the right category folder (including a
        // subfolder the user made inside it, like `Images/2019/`); a
        // recursive scan must not flatten that back out.
        if entry.path.starts_with(&dest_dir) {
            continue;
        }

        let reason = match &entry.extension {
            Some(ext) => format!("{category} file (.{ext})"),
            None => format!("{category} file (no extension)"),
        };

        operations.push(Operation::new(
            OperationKind::Move,
            entry.path.clone(),
            destination,
            reason,
            entry.size_bytes,
        ));
    }

    Plan::new(root.to_path_buf(), PlanMode::SortByType, operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn entry(name: &str, ext: Option<&str>) -> FileEntry {
        FileEntry {
            path: std::path::PathBuf::from(format!("/root/{name}")),
            file_name: name.to_string(),
            extension: ext.map(String::from),
            size_bytes: 10,
            modified: Utc::now(),
            created: None,
        }
    }

    #[test]
    fn routes_known_and_unknown_extensions() {
        let rules = CategoryRules::default_rules();
        let entries = vec![
            entry("photo.png", Some("png")),
            entry("weird.xyz", Some("xyz")),
        ];
        let plan = build_plan(Path::new("/root"), &entries, &rules);

        assert_eq!(plan.operations.len(), 2);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/Images/photo.png")
        );
        assert_eq!(
            plan.operations[1].destination,
            std::path::PathBuf::from("/root/Other/weird.xyz")
        );
    }

    #[test]
    fn skips_files_already_in_place() {
        let rules = CategoryRules::default_rules();
        let mut e = entry("photo.png", Some("png"));
        e.path = std::path::PathBuf::from("/root/Images/photo.png");
        let plan = build_plan(Path::new("/root"), &[e], &rules);
        assert_eq!(plan.operations.len(), 0);
    }

    /// Regression: with "include subfolders" on, `Images/2019/photo.png`
    /// was moved to `Images/photo.png`, flattening the user's own folders.
    #[test]
    fn leaves_files_in_subfolders_of_the_right_category_alone() {
        let rules = CategoryRules::default_rules();
        let mut nested = entry("photo.png", Some("png"));
        nested.path = std::path::PathBuf::from("/root/Images/2019/photo.png");
        let mut misfiled = entry("notes.txt", Some("txt"));
        misfiled.path = std::path::PathBuf::from("/root/Images/2019/notes.txt");

        let plan = build_plan(Path::new("/root"), &[nested, misfiled], &rules);

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(
            plan.operations[0].destination,
            std::path::PathBuf::from("/root/Documents/notes.txt")
        );
    }
}
