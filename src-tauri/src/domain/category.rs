use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDef {
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRules {
    pub categories: BTreeMap<String, CategoryDef>,
    pub other_folder_name: String,
}

impl CategoryRules {
    /// Returns the destination folder name for a given (lowercased) extension.
    pub fn categorize(&self, extension: Option<&str>) -> String {
        if let Some(ext) = extension {
            let ext = ext.to_lowercase();
            for (name, def) in &self.categories {
                if def.extensions.iter().any(|e| e.to_lowercase() == ext) {
                    return name.clone();
                }
            }
        }
        self.other_folder_name.clone()
    }

    pub fn default_rules() -> Self {
        let mut categories = BTreeMap::new();
        categories.insert(
            "Images".to_string(),
            CategoryDef {
                extensions: vec![
                    "jpg", "jpeg", "png", "gif", "bmp", "webp", "svg", "heic", "tiff",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            },
        );
        categories.insert(
            "Documents".to_string(),
            CategoryDef {
                extensions: vec![
                    "pdf", "doc", "docx", "txt", "md", "xls", "xlsx", "ppt", "pptx", "csv", "rtf",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            },
        );
        categories.insert(
            "Videos".to_string(),
            CategoryDef {
                extensions: vec!["mp4", "mkv", "avi", "mov", "wmv", "webm"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
        );
        categories.insert(
            "Audio".to_string(),
            CategoryDef {
                extensions: vec!["mp3", "wav", "flac", "aac", "ogg", "m4a"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
        );
        categories.insert(
            "Archives".to_string(),
            CategoryDef {
                extensions: vec!["zip", "rar", "7z", "tar", "gz"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
        );
        categories.insert(
            "Installers".to_string(),
            CategoryDef {
                extensions: vec!["exe", "msi", "msix"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            },
        );
        CategoryRules {
            categories,
            other_folder_name: "Other".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_known_extension() {
        let rules = CategoryRules::default_rules();
        assert_eq!(rules.categorize(Some("PNG")), "Images");
        assert_eq!(rules.categorize(Some("docx")), "Documents");
    }

    #[test]
    fn falls_back_to_other() {
        let rules = CategoryRules::default_rules();
        assert_eq!(rules.categorize(Some("xyz")), "Other");
        assert_eq!(rules.categorize(None), "Other");
    }
}
