use std::path::Path;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Language {
    Java
}

impl Language {
    pub fn is_source_file(&self, path: impl AsRef<Path>) -> bool {
        match self {
            Language::Java => path.as_ref().extension().unwrap_or_default() == "java"
        }
    }
}