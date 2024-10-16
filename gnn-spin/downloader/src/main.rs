use crate::schema::Project;

mod schema;


fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new().init()?;
    log::set_max_level(log::LevelFilter::Debug);
    
    let input_file = std::env::args().nth(1)
        .ok_or_else(|| anyhow::anyhow!("No input file provided"))?;
    let output_dir = std::env::args().nth(2)
        .ok_or_else(|| anyhow::anyhow!("No output directory provided"))?;
    
    log::info!("Reading projects from {}", input_file);
    log::info!("Writing projects to {}", output_dir);
    
    let output_dir = std::path::Path::new(&output_dir);
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }
    let file = std::fs::File::open(input_file)?;
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
