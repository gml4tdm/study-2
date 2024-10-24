//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Traits
//////////////////////////////////////////////////////////////////////////////////////////////////

// Note that these traits take &mut self in order to enable internal caching.

pub trait LogicalFileNameResolver {
    fn resolve(&mut self,
               file_path: &Path,
               parent: &Path,
               scan_root: &Path) -> anyhow::Result<Vec<(String, String, Option<(usize, usize)>)>>;
}

//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Java Implementation
//////////////////////////////////////////////////////////////////////////////////////////////////

pub struct JavaLogicalFileNameResolver;

impl JavaLogicalFileNameResolver {
    fn normalize_line(mut line: String) -> String {
        let o = line.clone();
        let mut i = 0;
        while let Some(start) = line.find("/*") {
            i += 1;
            if i > 100 {
                panic!("Infinite loop?: {} --> {} ({}, {:?})", o, line, start, line.find("*/"));
            }
            if let Some(stop) = line.find("*/") {
                if stop > start {
                    line.drain(start..stop + 2);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        line.trim().to_string()
    }
    
    fn extract_package_from_line(line: String, file_path: &Path) -> anyhow::Result<(String, String)> {
        let stripped = line.strip_prefix("package ")
            .unwrap()
            .to_string();
        match stripped.find(';') {
            None => {
                Err(anyhow::anyhow!(
                            "{}: Failed to parse package line remainder: {}",
                            file_path.display(),
                            stripped
                ))
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
                Ok((package, cls))
            }
        }
    }
    
    fn extract_entity_name(line: String,
                           entity_type: &str, 
                           file_path: &Path) -> anyhow::Result<String> {
        let stripped = if line.starts_with("public ") {
            line.strip_prefix("public")
                .unwrap()
                .to_string()
        } else if line.starts_with("protected") {
            line.strip_prefix("protected")
                .unwrap()
                .to_string()
        } else if line.starts_with("private") {
            line.strip_prefix("private")
                .unwrap()
                .to_string()    
        } else {
            line 
        };
        let stripped = stripped.chars()
            .skip_while(|c| c.is_whitespace())
            .collect::<String>();
        let stripped = stripped.strip_prefix(entity_type)
            .ok_or_else(|| anyhow::anyhow!("{}: Entity type removal error", file_path.display()))?
            .chars()
            .skip_while(|c| c.is_whitespace());
        let name = stripped.take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>();
        Ok(name)
    }
}


static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
static PATTERN2: OnceLock<regex::Regex> = OnceLock::new();


impl LogicalFileNameResolver for JavaLogicalFileNameResolver {
    fn resolve(&mut self,
               file_path: &Path,
               _parent: &Path,
               _scan_root: &Path) -> anyhow::Result<Vec<(String, String, Option<(usize, usize)>)>> {
        let file = std::fs::File::open(file_path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let content = String::from_utf8_lossy(buffer.as_slice());
        
        let mut package = None;
        let mut expected_class = None;
        let mut classes = Vec::new();
        for line in content.lines() {
            let o = line.clone();
            let line = line.trim().to_string();
            let line = Self::normalize_line(line);
            if line.starts_with("package ") {
                if package.is_some() {
                    // return Err(anyhow::anyhow!(
                    //     "{}: Found a second package declaration statement!", file_path.display()
                    // ));
                    // may happen in comments
                    continue;
                } else {
                    let (x, y) = Self::extract_package_from_line(line, file_path)?;
                    let _ = package.insert(x);
                    let _ = expected_class.insert(y);
                }
                continue;
            }
            let pat = PATTERN
                .get_or_init(|| 
                    regex::Regex::new(
                        r"^((public|protected|private)\s+)?(?<kind>interface|class|enum|@\s*interface|abstract\s+class|final\s+class|abstract\s+interface)\s*"
                    ).unwrap()
                );
            let pat2 = PATTERN2
                .get_or_init(||
                    regex::Regex::new(
                        r"^(?<kind>final|abstract)\s+((public|procected|private)\s+)?(?<kind2>class|interface)\s*"
                ).unwrap()
            );
            for c_pat in [pat, pat2] {
                if let Some(cap) = c_pat.captures(line.as_str()) {
                    let mut kind = cap.name("kind").expect("No kind capture group").as_str().to_string();
                    if let Some(specifier) = cap.name("kind2") {
                        kind = format!("{kind} {}", specifier.as_str());
                    }
                    let line = line.strip_prefix(cap.get(0).unwrap().as_str()).unwrap().chars();
                    let name = line.take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect::<String>();
                    classes.push((name, kind, None));
                    break;
                }
            }
            // for c in ["interface", "class", "enum", "@interface", "@ interface", "abstract class", "final class"] {
            //     let found = line.starts_with(c) ||
            //         line.starts_with(&format!("public {}", c)) ||
            //         line.starts_with(&format!("protected {}", c)) ||
            //         line.starts_with(&format!("private {}", c));
            //     if found {
            //         let entity= Self::extract_entity_name(line, c, file_path)?;
            //         classes.push((entity, c.to_string(), None));
            //         break;
            //     }
            // }
        }
        let prefix = package.ok_or_else(|| anyhow::anyhow!(
            "{}: Could not determine Java Package from file", file_path.display()
        ))?;
        let target = expected_class.unwrap();
        let found = classes.iter().find(|(name, _, _)| name == &target);
        if found.is_none() {
            if target.as_str() == "package-info" {
                log::info!("{}: Could not find class {} in file", file_path.display(), target);
            } else {
                log::warn!(
                    "{}: Could not find class {} in file", file_path.display(), target
                );
            }
        }
        Ok(
            classes.into_iter()
                .map(
                    |(name, kind, span)| (format!("{prefix}.{name}"), kind, span)
                )
                .collect()
        )
    }
}
