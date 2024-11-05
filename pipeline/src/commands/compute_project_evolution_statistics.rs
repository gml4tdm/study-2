use std::path::PathBuf;
use crate::graphs::loaders::load_graph_from_file;
use crate::statistics::project_evolution::get_project_evolution_statistics;
use crate::utils::versions::ExtractProjectInformation;


pub(crate) fn compute_project_evolution_statistics(files: Vec<PathBuf>,
                                                   output_path: PathBuf,
                                                   convert_to_package_graph: bool) -> anyhow::Result<()> {
    if files.is_empty() {
        log::warn!("No files provided!");
        return Ok(());
    }
    let project = files[0].extract_project()?.to_string();
    for path in files.iter().skip(1) {
        if path.extract_project()? != project {
            log::error!("Files are not in the same project!");
            return Ok(());
        }
    }
    // parse and order filenames 
    let mut version_pairs = files.into_iter()
        .map(|path| Ok((path.extract_version()?.to_string(), path)))
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    version_pairs.sort_by(
        |a, b|
            crate::utils::versions::cmp_versions(a.0.as_str(), b.0.as_str())
    );
    // 
    let mut versions = Vec::new();
    let mut graphs = Vec::new();
    for (version, path) in version_pairs {
        let graph = load_graph_from_file(path)?;
        versions.push(version);
        graphs.push(graph);
    }
    let stats = if convert_to_package_graph {
        get_project_evolution_statistics(
            &project,
            &versions,
            &graphs.into_iter()
                .map(|g| g.to_module_graph())
                .collect::<Vec<_>>()
        )
    } else {
        get_project_evolution_statistics(&project, &versions, &graphs)
    };
    serde_json::to_writer_pretty(std::fs::File::create(output_path)?, &stats)?;
    
    Ok(())
}