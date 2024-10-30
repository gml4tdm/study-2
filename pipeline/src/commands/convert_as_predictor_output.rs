use std::path::PathBuf;
use crate::replication::as_predictor::developer::read_as_predictor_output;

pub fn convert_as_predictor_output(input: PathBuf, output: PathBuf) -> anyhow::Result<()> {
    let converted = read_as_predictor_output(input)?;
    let file = std::fs::File::create(output)?;
    serde_json::to_writer_pretty(file, &converted)?;
    Ok(())
}
