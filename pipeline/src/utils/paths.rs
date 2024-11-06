use std::path::{Path, PathBuf};

pub trait ExtractFileName {
    fn extract_filename(&self) -> &str;
}

impl ExtractFileName for Path {
    fn extract_filename(&self) -> &str {
        self.file_stem()
            .unwrap_or_else(|| panic!("Failed to retrieve filename ({})", self.display()))
            .to_str()
            .unwrap_or_else(|| panic!("Failed to convert filename to string ({})", self.display()))
    }
}

impl ExtractFileName for PathBuf {
    fn extract_filename(&self) -> &str {
        self.as_path().extract_filename()
    }
}
