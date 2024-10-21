//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::io::BufRead;
use std::path::{Path, PathBuf};

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Traits
//////////////////////////////////////////////////////////////////////////////////////////////////

// Note that these traits take &mut self in order to enable internal caching.

pub trait LogicalFileNameResolver {
    fn resolve(&mut self,
               file_path: &Path,
               parent: &Path,
               scan_root: &Path) -> anyhow::Result<String>;
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Java Implementation
//////////////////////////////////////////////////////////////////////////////////////////////////

pub struct JavaLogicalFileNameResolver;

impl LogicalFileNameResolver for JavaLogicalFileNameResolver {
    fn resolve(&mut self,
               file_path: &Path,
               _parent: &Path,
               _scan_root: &Path) -> anyhow::Result<String> {
        let file = std::fs::File::open(file_path)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?.trim().to_string();
            if line.starts_with("package ") {
                let stripped = line.strip_prefix("package ")
                    .unwrap()
                    .to_string();
                match stripped.find(';') {
                    None => {
                        return Err(anyhow::anyhow!(
                            "{}: Failed to parse package line remainder: {}",
                            file_path.display(),
                            stripped
                        ));
                    }
                    Some(index) => {
                        let package = stripped[..index].to_string();
                        let cls = file_path.file_stem()
                            .ok_or_else(|| anyhow::anyhow!(
                                "{}: Could not get file stem", file_path.display()
                            ))?
                            .to_str()
                            .ok_or_else(|| anyhow::anyhow!(
                                "{}: Could not convert file stem to string", file_path.display()
                            ))?
                            .to_string();
                        return Ok(format!("{}.{}", package, cls));
                    }
                }
            }
        }
        Err(anyhow::anyhow!(
            "{}: Could not determine Java Package from file", file_path.display()
        ))
    }
}
