use std::path::PathBuf;
use crate::replication::as_predictor::developer::read_as_predictor_output;

pub fn convert_as_predictor_output(inputs: Vec<PathBuf>, output: PathBuf) -> anyhow::Result<()> {
    let mut result = Vec::new();
    for input in inputs { 
        let converted = read_as_predictor_output(input)?;
        result.extend(converted); 
    }
    let file = std::fs::File::create(output)?;
    serde_json::to_writer_pretty(file, &result)?;
    Ok(())
}
