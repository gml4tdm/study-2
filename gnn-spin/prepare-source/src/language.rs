use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java
}

impl Language {
    pub fn is_source_file(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        path.is_file() && 
            path.extension()
                .map(|ext| match self {
                    Language::Java => ext == "java"
                })
                .unwrap_or(false)
    }
}
