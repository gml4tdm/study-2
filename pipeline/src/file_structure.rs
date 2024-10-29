use crate::languages::Language;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceRoot {
    language: Language,
    path: String,
    root: Directory
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Directory {
    name: String,
    directories: Vec<Directory>,
    files: Vec<FileInfo>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    name: String,
}
