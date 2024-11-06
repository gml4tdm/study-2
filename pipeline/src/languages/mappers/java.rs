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
            r"(?x)^((public|protected|private)\s+)?(static\s+)?
            (?<kind>interface|class|enum|@\s*interface|abstract\s+class|final\s+class|abstract\s+interface)(\s+|\{)"
        ).unwrap()   
    )
}

fn get_java_type_pattern_2() -> &'static regex::Regex {
    JAVA_TYPE_PATTERN_2.get_or_init(||
        regex::Regex::new(
            r"^(?<kind>final|abstract)\s+((public|procected|private)\s+)?(?<kind2>class|interface)(\s+|\{)"
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
// Inner Class Resolver 
///////////////////////////////////////////////////////////////////////////////////////////////////


struct InnerClassHelper {
    expected: HashMap<String, HashMap<String, Class>>
}

struct Class {
    name: String,
    inner_classes: HashMap<String, Class>,
    found: bool 
}

impl InnerClassHelper {
    fn new(classes: HashSet<String>) -> Self {
        let mut expected = HashMap::new();
        for cls in classes {
            let (package, name) = cls.rsplit_once('.').unwrap();
            let parts = name.split('$').collect::<Vec<_>>();
            let container = expected.entry(package.to_string())
                .or_insert_with(HashMap::new);
            let mut current = match container.entry(parts[0].to_string()) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(Class {
                    name: parts[0].to_string(),
                    inner_classes: HashMap::new(),
                    found: false
                })
            };
            for part in parts.iter().skip(1) {
                current = match current.inner_classes.entry(part.to_string()) {
                    Entry::Occupied(e) => e.into_mut(),
                    Entry::Vacant(e) => e.insert(Class {
                        name: part.to_string(),
                        inner_classes: HashMap::new(),
                        found: false
                    })
                };
            }
        }
        Self { expected}
    }
    
    fn resolve_classes_from_file(&mut self, 
                                 prefix: &String,
                                 names: &[&String]) -> anyhow::Result<Option<Vec<String>>>
    {
        let mut result = Vec::new();
        // let package = self.expected.get_mut(prefix)
        //     .ok_or_else(|| anyhow::anyhow!("No package {} was expected", prefix))?;
        let package = match self.expected.get_mut(prefix) {
            Some(x) => x,
            None => return Ok(None)
        };
        
        // Subtle implementation detail we will rely on; 
        // classes are listed in the order they appear in the file,
        // which means that if we found a root class match,
        // the next classes are either inner class or a next root class.
        // This simplifies the search algorithm, since for every class,
        // it is either somewhere in the inner class structure of the previous 
        // root (check 1), or it must be a root class (check 2).
        // In particular, the first class must be a root class,
        
        // let mut current_root = package.get_mut(names[0])
        //     .ok_or_else(|| anyhow::anyhow!("No class {} found in package {}", names[0], prefix))?;
        let mut current_root = match package.get_mut(names[0]) {
            Some(x) => x,
            None => return Ok(None)
        };
        if current_root.found {
            return Err(anyhow::anyhow!("Class {} found twice in package {}", names[0], prefix));
        }
        current_root.found = true;
        result.push(current_root.name.clone());
        
        for name in names.into_iter().skip(1).copied() {
            match current_root.check_inner_class(name)? {
                Some(name) => {
                    result.push(name);
                }
                None => {
                    current_root = package.get_mut(name)
                        .ok_or_else(|| anyhow::anyhow!("No class {} found in package {}", name, prefix))?;
                    result.push(current_root.name.clone());
                }
            }
        }
        
        Ok(Some(result))
    }
}

impl Class {
    fn check_inner_class(&mut self, name: &str) -> anyhow::Result<Option<String>> {
        if name == self.name {
            if self.found {
                Err(anyhow::anyhow!("Class {} found twice in package {}", name, self.name))
            } else {
                Ok(Some(self.name.clone()))
            }
        } else if self.inner_classes.contains_key(name) {
            let name = self.inner_classes.get_mut(name)
                .unwrap()
                .check_inner_class(name)?
                .unwrap();
            Ok(Some(format!("{}${name}", self.name)))
        } else {
            for value in self.inner_classes.values_mut() {
                if let Some(name) = value.check_inner_class(name)? {
                    return Ok(Some(format!("{}${name}", self.name)));
                }
            }
            Ok(None)
        }
    }
}


///////////////////////////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////////////////////////
// Actual Resolver 
///////////////////////////////////////////////////////////////////////////////////////////////////

impl ObjectToSourceMapper for JavaClassToFileMapper {
    fn map(&self, _root: &Path, object: &str) -> anyhow::Result<ObjectLocation> {
        self.cache.get(object)
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve {}", object))
            .cloned()
    }
}

impl JavaClassToFileMapper {
    pub fn new(root: impl AsRef<Path>,
               included_classes: HashSet<String>) -> anyhow::Result<Self> {
        Ok(Self { cache: Self::resolve_all(root, included_classes)? })
    }
    
    fn resolve_all(root: impl AsRef<Path>, 
                   included: HashSet<String>) -> anyhow::Result<HashMap<String, ObjectLocation>> {
        log::info!("Resolving all classes in {}", root.as_ref().display());
        let mut helper = InnerClassHelper::new(included);
        let mut result = HashMap::new();
        let deep_root = root.as_ref().to_path_buf();
        Self::resolve_recursively(root, &mut  helper, &mut result, deep_root.as_path())?;
        Ok(result)
    }
    
    fn resolve_recursively(root: impl AsRef<Path>,
                           helper: &mut InnerClassHelper,
                           mapping: &mut HashMap<String, ObjectLocation>, 
                           deep_root: &Path) -> anyhow::Result<()> {
        log::info!("Checking directory {}...", root.as_ref().display());
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::resolve_recursively(path, helper, mapping, deep_root)?;
            } else if path.is_file() && Language::Java.is_source_file(&path) {
                match Self::resolve_file(&path, Self::relative_path(&path, deep_root))? {
                    Some((prefix, raw_classes)) => {
                        let names = raw_classes.iter()
                            .map(|cls| &cls.name)
                            .collect::<Vec<_>>();
                        let new_names = helper.resolve_classes_from_file(&prefix, &names)
                            .map_err(
                                |e| 
                                    e.context(format!("While resolving {}", path.display()))
                            )?;
                        
                        if new_names.is_none() {
                            for x in names {
                                log::warn!("Unexpected package or class in {prefix}.{x}, ignoring");
                            }
                            continue;
                        }
                        
                        let mut classes = Vec::new();
                        let stream = raw_classes.into_iter()
                            .zip(new_names.unwrap().into_iter());
                        for (mut original, new) in stream {
                            if original.name != new {
                                log::debug!("Mapped name {} to {}", original.name, new);
                                original.name = new;
                            }
                            classes.push(original);
                        }
                        
                        
                        for cls in classes {
                            log::debug!("Resolved item: {:?}", cls);
                            let key = format!("{}.{}", prefix, cls.name);
                            match mapping.entry(key) {
                                Entry::Occupied(e) => {
                                    log::error!(
                                        "Duplicate class found: {}.{} (previous = {:?}, new = {:?})", 
                                        prefix, cls.name, e.get(), cls
                                    );
                                    panic!("Duplicate class found: {}.{}", prefix, cls.name);
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
                    if name.is_empty() {
                        log::warn!("Found empty class name in line \"{}\", ignoring", line);
                        continue;
                    }
                    log::trace!("Found {} ({}) in line {}", name, kind, line);
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