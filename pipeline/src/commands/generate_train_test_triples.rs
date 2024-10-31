use std::path::PathBuf;
use crate::utils::versions::ExtractProjectInformation;

pub fn generate_train_test_triples(graph_files: Vec<PathBuf>,
                                   target_directory: PathBuf,
                                   only_common_nodes_for_training: bool) -> anyhow::Result<()>
{
    // Validate that all files are in the same project
    if graph_files.is_empty() {
        log::warn!("No files provided!");
        return Ok(());
    }
    let project = graph_files[0].extract_project()?.to_string();
    for path in graph_files.iter().skip(1) {
        if path.extract_project()? != project {
            log::error!("Files are not in the same project!");
            return Ok(());
        }
    }
    // parse and order filenames 
    let mut versions = graph_files.into_iter()
        .map(|path| Ok((path.extract_version()?.to_string(), path)))
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    versions.sort_by(
        |a, b|
            crate::utils::versions::cmp_versions(a.0.as_str(), b.0.as_str())
    );
    // Loop over all triples 
    std::fs::create_dir_all(&target_directory)?;
    for versions in versions.windows(3) {
        assert_eq!(versions.len(), 3);
        let v1 = &versions[0];
        let v2 = &versions[1];
        let v3 = &versions[2];
        log::info!("Generating triple for {project}: {}, {}, {}", v1.0, v2.0, v3.0);
        let triple = crate::datasets::triples::VersionTriple::from_files(
            v1.1.clone(), v2.1.clone(), v3.1.clone(), only_common_nodes_for_training
        )?;
        let target_path = target_directory.join(
            format!("{}-{}-{}-{}.json", project, v1.0, v2.0, v3.0)
        );
        let file = std::fs::File::create(target_path)?;
        serde_json::to_writer_pretty(file, &triple)?;
    }
    Ok(())
}