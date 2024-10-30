use std::path::{Path, PathBuf};

pub trait ExtractFileName {
    fn extract_filename(&self) -> &str;
}

impl ExtractFileName for Path {
    fn extract_filename(&self) -> &str {
        self.file_stem()
            .expect(
                format!("Failed to retrieve filename ({})", self.display()).as_str()
            )
            .to_str()
            .expect(
                format!("Failed to convert filename to string ({})", self.display()).as_str()
            )
    }
}

impl ExtractFileName for PathBuf {
    fn extract_filename(&self) -> &str {
        self.as_path().extract_filename()
    }
}
