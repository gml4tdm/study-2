//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Imports
//////////////////////////////////////////////////////////////////////////////////////////////////

use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;


//////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////
// Java Implementation
//////////////////////////////////////////////////////////////////////////////////////////////////

pub struct JavaLogicalFileNameResolver;


#[derive(Debug, Clone, serde::Serialize)]
pub struct EntityInfo {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
}

static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
static PATTERN2: OnceLock<regex::Regex> = OnceLock::new();

static PACKAGE_PATTERN: OnceLock<regex::Regex> = OnceLock::new();

impl JavaLogicalFileNameResolver {
    fn normalize_line(mut line: String) -> String {
        while let Some(start) = line.find("/*") {
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

    pub fn resolve(&mut self, file_path: &Path, root_dir: &Path) -> anyhow::Result<Option<(String, Vec<EntityInfo>)>> {
        let file = std::fs::File::open(file_path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let content = String::from_utf8_lossy(buffer.as_slice());

        let mut package = None;
        let mut expected_class = None;
        let mut classes = Vec::new();
        for line in content.lines() {
            let line = line.trim().to_string();
            let line = Self::normalize_line(line);
            let p_pattern = PACKAGE_PATTERN.get_or_init(||
                regex::Regex::new(
                    r"^package\s+(?<package>[a-zA-Z0-9_.]+);"
                ).unwrap()
            );
            if let Some(cap) = p_pattern.captures(line.as_str()) {
                if package.is_some() {
                    continue;
                }
                let package_name = cap.name("package")
                    .expect("No package capture group").as_str().to_string();
                let _ = package.insert(package_name);
                let cls = file_path.file_stem()
                    .ok_or_else(|| anyhow::anyhow!(
                                "{}: Could not get file stem", file_path.display()
                            ))?
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!(
                                "{}: Could not convert file stem to string", file_path.display()
                            ))?
                    .to_string();
                let _ = expected_class.insert(cls);
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
        }
        let prefix = match package {
            None => {
                log::warn!("{}: Could not determine Java Package from file", file_path.display());
                return Ok(None);
            }
            Some(x) => x
        };
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
        let loc_path = file_path.to_path_buf()
            .display()
            .to_string()
            .strip_prefix(root_dir.display().to_string().as_str())
            .expect("Failed to strip root directory from path")
            .strip_prefix('/')
            .expect("Failed to strip leading slash from path")
            .to_string();
        let classes = classes.into_iter()
            .map(|(name, kind, span)| EntityInfo {
                name,
                kind,
                path: loc_path.clone(),
                byte_start: span.map(|(start, _)| start),
                byte_end: span.map(|(_, end)| end)
            })
            .collect();
        Ok(Some((prefix, classes)))
    }
}
