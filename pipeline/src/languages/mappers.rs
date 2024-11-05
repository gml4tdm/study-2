use std::path::{Path, PathBuf};

mod java;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectLocation {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
}

pub trait ObjectToSourceMapper {
    fn map(&mut self, root: impl AsRef<Path>, object: &str) -> anyhow::Result<ObjectLocation>;
}
