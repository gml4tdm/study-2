use std::path::PathBuf;
use crate::datasets::co_change::extract_co_change_history;
use crate::processing::history::{ClassChangeInfo, History};

pub fn generate_co_change_features(input_file: PathBuf,
                                   output_file: PathBuf) -> anyhow::Result<()>
{
    let file = std::fs::File::open(input_file)?;
    let reader = std::io::BufReader::new(file);
    let history: History<ClassChangeInfo> = serde_json::from_reader(reader)?;
    let features = extract_co_change_history(history);
    let file = std::fs::File::create(output_file)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &features)?;
    Ok(())
}