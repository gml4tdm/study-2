use std::collections::HashMap;
use std::path::PathBuf;
use crate::GraphFormat;

pub struct SourcePair {
    pub project: String,
    pub version: String,
    pub code: PathBuf,
    pub graph: PathBuf
}


pub fn find_source_pairs(graph_directory: PathBuf,
                         source_directory: PathBuf,
                         graph_format: GraphFormat, 
                         project_name_mapping: HashMap<String, String>) -> anyhow::Result<Vec<SourcePair>> {
    let mut pairs = Vec::new();
    for entry in std::fs::read_dir(source_directory)? {
        let path = entry?.path();
        let project_name = path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Could not get project name"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Could not convert project name to string"))?
            .to_string();
        log::info!("Processing project: {}", project_name);
        for inner_entry in std::fs::read_dir(path)? {
            let inner_path = inner_entry?.path();
            let project_version = inner_path.file_name()
                .ok_or_else(|| anyhow::anyhow!("Could not get project version"))?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Could not convert project version to string"))?
                .to_string();
            log::info!("Found project version: {}", project_version);
            let mapped = project_name_mapping.get(&project_name)
                .unwrap_or(&project_name);
            let graph_path = graph_directory
                .join(project_name.as_str())
                .join(format!("{mapped}-{project_version}.{}", graph_format.extension()));
            log::info!("Inferred graph path: {}", graph_path.display());
            if !graph_path.exists() {
                log::error!("Graph file does not exist: {}", graph_path.display());
                return Err(anyhow::anyhow!("Graph file does not exist: {}", graph_path.display()));
            }
            pairs.push(SourcePair {
                project: project_name.clone(),
                version: project_version,
                code: inner_path,
                graph: graph_path
            });
        }
    }
    Ok(pairs)
}