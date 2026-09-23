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

        // Already sorted into the right place; nothing to do.
        if entry.path == destination {
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
}
