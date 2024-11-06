use std::path::Path;

pub mod java;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectLocation {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
}

pub trait ObjectToSourceMapper {
    fn map(&self, root: &Path, object: &str) -> anyhow::Result<ObjectLocation>;
}
