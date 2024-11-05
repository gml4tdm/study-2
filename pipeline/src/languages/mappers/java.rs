use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;
use crate::languages::Language;
use crate::utils::paths::ExtractFileName;
use super::{ObjectLocation, ObjectToSourceMapper};

#[derive(Debug)]
pub struct JavaClassToFileMapper {
    cache: HashMap<String, ObjectLocation>
}

///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Regex 
///////////////////////////////////////////////////////////////////////////////////////////////////

static JAVA_TYPE_PATTERN_1: OnceLock<regex::Regex> = OnceLock::new();
static JAVA_TYPE_PATTERN_2: OnceLock<regex::Regex> = OnceLock::new();
static JAVA_PACKAGE_PATTERN: OnceLock<regex::Regex> = OnceLock::new();

fn get_java_type_pattern_1() -> &'static regex::Regex {
    JAVA_TYPE_PATTERN_1.get_or_init(||
        regex::Regex::new(
            r"(?x)^((public|protected|private)\s+)?
            (?<kind>interface|class|enum|@\s*interface|abstract\s+class|final\s+class|abstract\s+interface)\s*"
        ).unwrap()   
    )
}

fn get_java_type_pattern_2() -> &'static regex::Regex {
    JAVA_TYPE_PATTERN_2.get_or_init(||
        regex::Regex::new(
            r"^(?<kind>final|abstract)\s+((public|procected|private)\s+)?(?<kind2>class|interface)\s*"
        ).unwrap()
    )
}

fn get_java_package_pattern() -> &'static regex::Regex {
    JAVA_PACKAGE_PATTERN.get_or_init(||
        regex::Regex::new(
            r"^package\s+(?<package>[a-zA-Z0-9_.]+);"
        ).unwrap()
    )
}


///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Actual Resolver 
///////////////////////////////////////////////////////////////////////////////////////////////////

impl ObjectToSourceMapper for JavaClassToFileMapper {
    fn map(&mut self, _root: impl AsRef<Path>, object: &str) -> anyhow::Result<ObjectLocation> {
        self.cache.get(object)
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve {}", object))
            .cloned()
    }
}

impl JavaClassToFileMapper {
    pub fn new(root: impl AsRef<Path>,
               included_classes: Option<HashSet<String>>) -> anyhow::Result<Self> {
        Ok(Self { cache: Self::resolve_all(root, included_classes)? })
    }
    
    fn resolve_all(root: impl AsRef<Path>, 
                   included: Option<HashSet<String>>) -> anyhow::Result<HashMap<String, ObjectLocation>> {
        let mut result = HashMap::new();
        let included_set = included.unwrap_or_default();
        Self::resolve_recursively(root, &included_set, &mut result)?;
        Ok(result)
    }
    
    fn resolve_recursively(root: impl AsRef<Path>,
                           _included: &HashSet<String>, 
                           mapping: &mut HashMap<String, ObjectLocation>) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::resolve_recursively(path, _included, mapping)?;
            } else if path.is_file() && Language::Java.is_source_file(&path) {
                match Self::resolve_file(&path, Self::relative_path(&path, &root))? {
                    Some((prefix, classes)) => {
                        for cls in classes {
                            let key = format!("{}.{}", prefix, cls.name);
                            match mapping.entry(key) {
                                Entry::Occupied(_e) => {
                                    log::error!(
                                        "Duplicate class found: {}.{} ({:?})", prefix, cls.name, cls
                                    );
                                    panic!(
                                        "Duplicate class found: {}.{} ({:?})", prefix, cls.name, cls
                                    );
                                }
                                Entry::Vacant(e) => {
                                    e.insert(cls);
                                }
                            }
                        }
                    },
                    None => {
                        log::warn!("File outside of package tree: {}", path.display());
                    }
                }
            }
        }
        Ok(())   
    }
    
    fn relative_path(path: impl AsRef<Path>, to: impl AsRef<Path>) -> String {
        path.as_ref().to_path_buf()
            .display()
            .to_string()
            .strip_prefix(to.as_ref().display().to_string().as_str())
            .expect("Failed to strip root directory from path")
            .strip_prefix('/')
            .expect("Failed to strip leading slash from path")
            .to_string()
    }

    fn resolve_file(file: impl AsRef<Path>,
                    relative_path: String) -> anyhow::Result<Option<(String, Vec<ObjectLocation>)>> {
        let path = file.as_ref();
        let mut package = None;
        let expected_class = file.as_ref().extract_filename();
        let mut classes = Vec::new();
        for line in Self::read_file(path)?.lines() {
            let line = line.trim().to_string();
            let line = Self::normalize_line(line);
            // Check for package 
            if let Some(cap) = get_java_package_pattern().captures(&line) {
                if package.is_none() {
                    let package_name = cap.name("package").unwrap()
                        .as_str().to_string();
                    let _ = package.insert(package_name);
                }
                continue;
            }
            // Check for class 
            for pat in [get_java_type_pattern_1(), get_java_type_pattern_2()] {
                if let Some(cap) = pat.captures(&line) {
                    let (name, kind) = Self::get_class_name_and_kind(&line, cap)?;
                    classes.push((name, kind, None));
                    break;
                }
            }
        }
        let found = classes.iter()
            .find(|(name, _, _)| name == expected_class);
        if found.is_none() {
            if expected_class == "package-info" {
                log::info!(
                    "{}: Could not find class {} in file", path.display(), expected_class
                );
            } else {
                log::warn!(
                    "{}: Could not find class {} in file", path.display(), expected_class
                );
            }
        }
        let prefix = match package {
            Some(x) => x,
            None => { return Ok(None); }
        };
        let classes = classes.into_iter()
            .map(|(name, kind, span)| ObjectLocation {
                name,
                kind,
                path: relative_path.clone(),
                byte_start: span.map(|(start, _)| start),
                byte_end: span.map(|(_, end)| end)
            })
            .collect();
        Ok(Some((prefix, classes)))
    }

    fn read_file(file_path: impl AsRef<Path>) -> anyhow::Result<String> {
        let file = std::fs::File::open(file_path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let content = String::from_utf8_lossy(buffer.as_slice());
        Ok(content.to_string())
    }

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

    fn get_class_name_and_kind(line: &str, cap: regex::Captures) -> anyhow::Result<(String, String)> {
        let mut kind = cap.name("kind").expect("No kind capture group")
            .as_str().to_string();
        if let Some(specifier) = cap.name("kind2") {
            kind = format!("{kind} {}", specifier.as_str());
        }
        let line = line.strip_prefix(cap.get(0).unwrap().as_str())
            .unwrap()
            .chars();
        let name = line
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>();
        Ok((name, kind))
    }
}