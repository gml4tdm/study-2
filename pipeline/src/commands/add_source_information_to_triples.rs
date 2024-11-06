use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::datasets::triples::{Graph, VersionTriple};
use crate::languages::Language;
use crate::languages::mappers::java::JavaClassToFileMapper;
use crate::languages::mappers::ObjectToSourceMapper;

pub fn add_source_information_to_triples(input_files: Vec<PathBuf>,
                                         source_directory: PathBuf,
                                         output_directory: Option<PathBuf>) -> anyhow::Result<()>
{
    if let Some(dir) = output_directory.as_ref() {
        std::fs::create_dir_all(dir)?;
    }
    for filename in input_files {
        let file = std::fs::File::open(&filename)?;
        let reader = std::io::BufReader::new(file);
        let mut triple = serde_json::from_reader(reader)?;
        add_source_information_to_triple(&mut triple, source_directory.as_path())?;
        if let Some(dir) = output_directory.as_ref() {
            let mut file = std::fs::File::create(dir.join(filename.file_name().unwrap()))?;
            serde_json::to_writer_pretty(&mut file, &triple)?;
        } else {
            let mut file = std::fs::File::create(filename)?;
            serde_json::to_writer_pretty(&mut file, &triple)?;
        }
    }   
    Ok(())
}

fn add_source_information_to_triple(triple: &mut VersionTriple,
                                    source_directory: &Path) -> anyhow::Result<()> 
{
    let path_1 = source_directory
        .join(triple.project())
        .join(triple.version_1());
    let path_2 = source_directory
        .join(triple.project())
        .join(triple.version_2());
    // let path_3 = source_directory
    //     .join(triple.project())
    //     .join(triple.version_3());
    let mut classes_1 = HashSet::new();
    let mut classes_2 = HashSet::new();
    for g in [&triple.training_graph(), &triple.test_graph()] {
        for cls in g.classes() {
            if cls.versions().contains(&1) {
                classes_1.insert(format!("{}.{}", cls.package(), cls.name()));
            }
            if cls.versions().contains(&2) {
                classes_2.insert(format!("{}.{}", cls.package(), cls.name()));
            }
        }
    }

    let resolvers: HashMap<u8, Box<dyn ObjectToSourceMapper>> = match triple.metadata().language {
        Language::Java => {
            HashMap::from([
                (b'\x01', Box::new(JavaClassToFileMapper::new(&path_1, classes_1)?) as Box<dyn ObjectToSourceMapper>),
                (b'\x02', Box::new(JavaClassToFileMapper::new(&path_2, classes_2)?) as Box<dyn ObjectToSourceMapper>),
                //(b'\x03', Box::new(JavaClassToFileMapper::new(&path_3, None)?) as Box<dyn ObjectToSourceMapper>)
            ])
        }
    };
    let paths = HashMap::from([
        (b'\x01', path_1),
        (b'\x02', path_2),
        //(b'\x03', path_3),
    ]);
    add_source_information_to_graph(triple.training_graph_mut(), &resolvers, &paths)?;
    add_source_information_to_graph(triple.test_graph_mut(), &resolvers, &paths)?;
    Ok(())
}

fn add_source_information_to_graph(
    graph: &mut Graph,
    resolvers: &HashMap<u8, Box<dyn ObjectToSourceMapper>>,
    roots: &HashMap<u8, PathBuf>) -> anyhow::Result<()>
{
    let mut classes_by_node: HashMap<String, Vec<String>> = HashMap::new();
    for cls in graph.classes() {
        classes_by_node.entry(cls.package().to_string())
            .or_default()
            .push(cls.name().to_string());
    }
    let empty = Vec::new();
    let mut errors = Vec::new();
    let pattern = regex::Regex::new(r".+\$[0-9]+$")?;
    for node in graph.nodes_mut() {
        // let v = *node.versions().iter().max().expect("No versions in node");
        let mut versions = node.versions().clone();
        versions.sort();
        versions.reverse();
        
        for cls in classes_by_node.get(node.name()).unwrap_or(&empty) {
            if pattern.is_match_at(cls, 0) {
                log::warn!("Skipping resolving anonymous inner class {}", cls);
                continue;
            }
            
            let mut failed = true;
            for v in versions.iter() {
                let resolver = resolvers.get(&v).expect("No resolver for version");
                let root = roots.get(&v).expect("No root for version");
                let source = match resolver.map(root.as_path(), &format!("{}.{}", node.name(), cls)) {
                    Ok(x) => x,
                    Err(_e) => {
                        continue;
                    }
                };
                failed = false;
                node.files_mut().insert(cls.to_string(), source);
            }
            if failed {
                log::error!("Failed to map {}.{}", node.name(), cls);
                errors.push(format!("Failed to map {}", cls));
            }
        }
    }
    if !errors.is_empty() {
        log::error!("Errors while mapping source files:");
        for error in errors {
            log::error!("  * {}", error);
        }
        anyhow::bail!("Failed to map source files");
    }
    Ok(())
}
