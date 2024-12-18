use std::collections::HashMap;
use std::path::PathBuf;
use crate::datasets::co_change::CoChangeDataset;
use crate::graphs::loaders::load_graph_from_file;
use crate::utils::versions::ExtractProjectInformation;
use crate::datasets::co_change_2::generate_co_change_features_2;

pub fn finalise_co_change_features(change_file: PathBuf,
                                   graph_files: Vec<PathBuf>,
                                   output_path: PathBuf) -> anyhow::Result<()> 
{
    let file = std::fs::File::open(change_file)?;
    let reader = std::io::BufReader::new(file);
    let change_data: CoChangeDataset = serde_json::from_reader(reader)?;
    
    let mut graphs = HashMap::new();
    for file in graph_files {
        let version = file.extract_version()?.to_string();
        let graph = load_graph_from_file(file)?;
        let mut parts = version.split('.');
        let major = parts.next().unwrap().to_string();
        let minor = parts.next().unwrap().to_string();
        graphs.insert((major, minor), graph);
    }
    
    let result = generate_co_change_features_2(change_data, graphs);
    
    std::fs::create_dir_all(output_path.parent().unwrap())?;
    let file = std::fs::File::create(output_path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &result)?;
    
    Ok(())
}
