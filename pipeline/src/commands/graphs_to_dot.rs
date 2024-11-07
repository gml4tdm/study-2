use std::path::PathBuf;
use crate::graphs::loaders::load_graph_from_file;
use crate::utils::paths::ExtractFileName;

pub fn graphs_to_dot(input_files: Vec<PathBuf>,
                     output_directory: PathBuf,
                     package_diagrams: bool) -> anyhow::Result<()> 
{
    // Make output directory if it doesn't exist
    log::debug!("Generating DOT files");
    log::debug!("Generating output directory");
    std::fs::create_dir_all(&output_directory)?;
    
    // Generate DOT files
    for input_file in input_files {
        log::info!("Processing file {}...", input_file.display());
        let class_graph = load_graph_from_file(&input_file)?;
        let dot_source = if !package_diagrams {
            class_graph.to_dot()
        } else {
            class_graph.to_module_graph().to_dot()
        };
        let filename = input_file.extract_filename();
        let output_path = output_directory.join(format!("{}.dot", filename));
        log::info!("Writing to {}", output_path.display());
        std::fs::write(output_path, dot_source)?;
    }
    Ok(())
}
