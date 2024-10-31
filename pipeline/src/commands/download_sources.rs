use std::path::PathBuf;

use crate::source_downloader::Project;

pub fn download_sources(spec_file: PathBuf, output_directory: PathBuf) -> anyhow::Result<()> {
    log::info!("Reading projects from {}", spec_file.display());
    log::info!("Writing projects to {}", output_directory.display());

    let output_dir = std::path::Path::new(&output_directory);
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }
    let file = std::fs::File::open(spec_file)?;
    let reader = std::io::BufReader::new(file);
    let projects = serde_json::from_reader::<_, Vec<Project>>(reader)?;

    log::info!("Found {} projects", projects.len());
    for project in &projects {
        log::info!(
            "Found project {} with {} versions", 
            project.name.as_str(), 
            project.versions.len()
        );
    }

    for project in projects {
        project.download_all_versions(output_dir)?;
    }
    Ok(())
}