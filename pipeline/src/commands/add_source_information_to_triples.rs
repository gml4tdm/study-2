use std::path::{Path, PathBuf};
use crate::datasets::triples::VersionTriple;

pub fn add_source_information_to_triples(input_files: Vec<PathBuf>,
                                         source_directory: PathBuf,
                                         output_directory: Option<PathBuf>) -> anyhow::Result<()>
{
    if let Some(dir) = output_directory.as_ref() {
        std::fs::create_dir_all(dir)?;
    }
    for filename in input_files {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        let mut triple = serde_json::from_reader(reader)?;
        add_source_information_to_triple(&mut triple, source_directory.as_path())?;
    }   
    Ok(())
}


fn add_source_information_to_triple(triple: &mut VersionTriple,
                                    source_directory: &Path) -> anyhow::Result<()> 
{
    let path = source_directory
        .join(triple.project.as_str())
        .join(triple.version.as_str());
    Ok(())
}