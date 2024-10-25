use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use crate::language::Language;
use std::path::{Path, PathBuf};
use crate::resolver::{EntityInfo, JavaLogicalFileNameResolver};
use crate::schema::DependencyGraphRoot;


#[derive(Debug, Clone, serde::Serialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub package: String,
    pub entities: Vec<EntityInfo>
}



pub fn select_sources_from_graph(graph: DependencyGraphRoot,
                                 language: Language,
                                 code_path: PathBuf) -> anyhow::Result<Vec<FileInfo>> {
    let mut known_types: HashMap<String, HashSet<String>> = HashMap::new();
    for container in graph.context.containers.iter() {
        for namespace in container.namespaces.iter() {
            for entity in namespace.r#types.iter() {
                let entity_name = entity.name
                    .strip_prefix(&namespace.name)
                    .expect("Failed to strip namespace from entity name")
                    .strip_prefix('.')
                    .expect("Failed to strip leading dot from entity name")
                    .to_string();
                match known_types.entry(namespace.name.clone()) {
                    Entry::Occupied(mut e) => {
                        e.get_mut().insert(entity_name);
                    }
                    Entry::Vacant(e) => {
                        e.insert(HashSet::from_iter([entity_name]));
                    }
                }
            }
        }
    }
    let root = code_path.clone();
    walk_directory(code_path, language, &known_types, root.as_path())
}

fn walk_directory(path: PathBuf,
                  language: Language,
                  known: &HashMap<String, HashSet<String>>,
                  root: &Path) -> anyhow::Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        if language.is_source_file(path.as_path()) {
            if let Some((package, entities)) = JavaLogicalFileNameResolver.resolve(&path, root)? { 
                let col = match known.get(&package) {
                    None => { continue; }
                    Some(col) => col
                };
                let include = col.iter().any(|x| entities.iter().any(|y| y.name == *x));
                if !include {
                    continue;
                }
                let info = FileInfo {
                    path,
                    package,
                    entities
                };
                files.push(info);   
            }
        } else if path.is_dir() {
            let sub_files = walk_directory(path, language, known, root)?;
            files.extend(sub_files);
        }
    }
    Ok(files)
}
